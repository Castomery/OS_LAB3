use core::ptr::null_mut;
use crate::{print, println};
use crate::vga_buf::SCREEN;
use pc_keyboard::DecodedKey;
use lazy_static::lazy_static;


const MAX_CHILDREN: usize = 20;
const DELETE_INDEX: usize = MAX_CHILDREN + 1;
const BUF_SIZE:usize = (25*80) as usize; 

lazy_static! {
    static ref SH: spin::Mutex<Shell> = spin::Mutex::new({
        let mut sh = Shell::new();
        sh
    });
}

pub fn handle_keyboard_interrupt(key: DecodedKey) 
{
    match key { 
        DecodedKey::Unicode(c) => SH.lock().on_key_pressed(c as u8),
        DecodedKey::RawKey(rk) => {}
    }
} 

pub fn print_special_symbol()
{ 
    print!(" $ ");
}

pub fn get_command(arr:[u8;80],buf_len:usize)-> [u8;10]
{
 let mut command:[u8;10] = [b'\0';10];

 for i in 0..10
 {
    if arr[i]==b' '
    {
        break;
    }

    command[i] = arr[i];
 }
 return command;
}

pub fn get_value(arr: [u8; 80],buf_len:usize)->[u8;70]
{
    let mut value:[u8;70] = [b'\0';70];
    let mut j = 0;
    let mut is_value = false;

    for i in 0..buf_len  
    {
        if arr[i] == b' ' 
        {
            is_value = true;
            continue;
        }

        if is_value
        {
         value[j] = arr[i];
         j+=1;   
        }
      
    }

    return value;
}

pub fn check_command(str_for_compare: &str, arr: [u8; 10]) -> bool 
{
    let mut is_valid = true;

    let mut i = 0;
    for symbol in str_for_compare.bytes() {
        if symbol != arr[i] {
            is_valid = false;
        }
        i += 1;
    }
    return is_valid;
}

struct Dirs 
{
    dirs: [Dir; 100],
 }
 
 #[derive(Debug, Clone, Copy)]
 struct Dir 
 {
    index: usize,
    name: [u8; 10],
    parent_index: usize,
    child_count: u8,
    child_indexes: [usize; MAX_CHILDREN],
    files_indexes: [usize; MAX_CHILDREN]
 }

 #[derive(Debug, Clone, Copy)]
 struct File 
 {
    index: usize,
    name: [u8; 10],
    count_lines: usize,
    folder_index: usize,
    content: [u8; BUF_SIZE],
}

struct FileList 
{
    files: [File; 100],
}
 

struct Shell 
{
    buf: [u8; 80],
    buf_len: usize,
    directories: Dirs,
    files_list: FileList,
    cur_dir: usize,
    is_editing_file: bool,
    current_editing_file: usize,
}

impl Shell 
{
    pub fn new() -> Shell 
    {
        let mut shell:Shell = Shell {
            buf: [0; 80],
            buf_len: 0,
            directories: Dirs {
                dirs: ([Dir {
                    index: DELETE_INDEX,
                    name: [b' '; 10],
                    parent_index: 0,
                    child_count: 0,
                    child_indexes: [DELETE_INDEX; MAX_CHILDREN],
                    files_indexes: [DELETE_INDEX; MAX_CHILDREN],
                }; 100]),
            },
            cur_dir: 0,
            files_list: FileList {
                files: [File {
                    index: DELETE_INDEX,
                    name: [b'\0'; 10],
                    count_lines: 0,
                    folder_index: DELETE_INDEX,
                    content: [b' '; BUF_SIZE],
                }; 100],
            },
            is_editing_file: false,
            current_editing_file: DELETE_INDEX,
        };

        shell.directories.dirs[0] = Dir{
            index: 0,
            name: [
                b'r', b'o', b'o', b't', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
            ],
            parent_index: 0,
            child_count: 0,
            child_indexes: [DELETE_INDEX; MAX_CHILDREN],
            files_indexes: [DELETE_INDEX; MAX_CHILDREN],
        };

        return shell;
    }

    pub fn on_key_pressed(&mut self, key: u8) 
    {
        match key 
        {
            b'\n' => 
            {
                let command = get_command(self.buf, self.buf_len);
                let value: [u8;70] = get_value(self.buf, self.buf_len);

                self.execute_command(command,value);
                self.buf_len = 0;
                println!();
                print!(" $ ");
            }
            8 =>
            {
                SCREEN.lock().remove_symbol(3);

                if self.buf_len > 0 {
                    self.buf_len -= 1;
                }

                self.buf[self.buf_len] = 0;
            }
            _ => {
                self.buf[self.buf_len] = key;
                self.buf_len += 1;
                print!("{}", key as char);
            }
        }
    }

    fn execute_command(&mut self, command:[u8;10], value:[u8;70])
    {
        if check_command("cur_dir", command) 
        {
            self.cur_dir(self.directories.dirs[self.cur_dir]);
        } 
        else if check_command("make_dir", command) 
        {
            self.make_dir(value);
        }
        else if check_command("change_dir",command)
        {
            self.change_dir(value);
        }
        else if check_command("remove_dir",command)
        {
            self.remove_dir(value);
        }
        else if check_command("dir_tree",command)
        {
            println!();
            print!("/{}",
                core::str::from_utf8(&self.directories.dirs[self.cur_dir].name)
                    .unwrap()
                    .trim_matches('\0')
            );

            self.dir_tree(self.directories.dirs[self.cur_dir],1);
        }
        else if check_command("clear",command)
        {
            SCREEN.lock().clear();
        }
        else{
            println!();
            print!(
                "\n[Error] Command \"{}\" is not supported!",
                core::str::from_utf8(&command.clone())
                    .unwrap()
                    .trim_matches('\0')
            );
        }


    }

    fn cur_dir(&mut self, cur_dir: Dir)
    {
        println!();
        print!(
            "/{}",
            core::str::from_utf8(&cur_dir.name.clone())
                .unwrap()
                .trim_matches('\0')
        );
    }

    fn make_dir(&mut self, value: [u8; 70]) 
    {
        let mut name_size = 0;
        for i in 0..70 
        {
            if value[i] == b'\0' 
            {
                break;
            }
            name_size += 1;
        }

        if name_size > 10 
        {
            print!("\n[Error] Invalid directory name. Max size is 10 characters");
            return;
        }

        let mut dir_index = DELETE_INDEX;

        for i in 0..100 
        {
            if self.directories.dirs[i].index == DELETE_INDEX 
            {
                dir_index = i;
                break;
            }
        }

        if dir_index == DELETE_INDEX 
        {
            print!("\n[Error] Can`t be created in this directory!");
            return;
        }

        let mut available_index = DELETE_INDEX;

        for i in 0..MAX_CHILDREN
        {
            if self.directories.dirs[self.cur_dir].child_indexes[i] == DELETE_INDEX
            {
                available_index = i;
                break;
            }
        }

        if available_index == DELETE_INDEX
        {
            print!("\n[Error] Can`t be created in this directory!");
            return;
        }

        let mut new_directory: Dir = Dir {
            index: dir_index,
            name: [b'\0'; 10],
            parent_index: self.cur_dir,
            child_count: 0,
            child_indexes: [DELETE_INDEX; MAX_CHILDREN],
            files_indexes: [DELETE_INDEX; 20],
        };

        for i in 0..10 {
            new_directory.name[i] = value[i];
        }

        self.directories.dirs[dir_index] = new_directory;
        self.directories.dirs[self.cur_dir].child_indexes[available_index] = dir_index;
        self.directories.dirs[self.cur_dir].child_count += 1;

        print!(
            "\n[Ok] Created new dir \"{}\"",
            core::str::from_utf8(&new_directory.name.clone())
                .unwrap()
                .trim_matches('\0')
        );
    }

    fn change_dir(&mut self, arg: [u8; 70]) 
    {
        if arg[0] == b'.' 
        {
            self.cur_dir = self.directories.dirs[self.cur_dir].parent_index;
            return;
        }

        let cur_dir = self.directories.dirs[self.cur_dir];

        for dir_index in cur_dir.child_indexes 
        {
            let mut has_same_name = true;

            for i in 0..70 
            {
                if arg[i] == b'\0' 
                {
                    break;
                }

                if i == 10 
                {
                    print!("[Error] Invalid directory name. Max size is 10 characters");
                    return;
                }

                if self.directories.dirs[dir_index].name[i] != arg[i] 
                {
                    has_same_name = false;
                    break;
                }
            }

            if has_same_name {
                self.cur_dir = self.directories.dirs[dir_index].index;
                println!();
                print!(
                    "\n[Ok] Changed current dir to \"{}\"",
                    core::str::from_utf8(&self.directories.dirs[self.cur_dir].name.clone())
                        .unwrap()
                        .trim_matches('\0')
                );
                return;
            }
        }

        print!(
            "\nFolder \"{}\" doesn't exist!",
            core::str::from_utf8(&arg.clone())
                .unwrap()
                .trim_matches('\0')
        )
    }

    fn remove_dir(&mut self, dir_name: [u8; 70]) 
    {
        let cur_dir = self.directories.dirs[self.cur_dir];
        let mut has_same_name = true;

        for i in 0..cur_dir.child_count as usize 
        {
            let dir_to_check = self.directories.dirs[cur_dir.child_indexes[i]];

            for j in 0..10 
            {
                if dir_to_check.name[j] != dir_name[j] 
                {
                    has_same_name = false;
                    break;
                }
            }

            if !has_same_name 
            {
                continue;
            }

            if self.directories.dirs[dir_to_check.index].child_count > 0 
            {
                println!();
                print!("[Error] Count of children must be 0");
                return;
            }

            self.directories.dirs[self.cur_dir].child_count -= 1;

            self.directories.dirs[dir_to_check.index] = Dir {
                index: DELETE_INDEX,
                name: [b' '; 10],
                parent_index: DELETE_INDEX,
                child_count: DELETE_INDEX as u8,
                child_indexes: [DELETE_INDEX; MAX_CHILDREN],
                files_indexes: [DELETE_INDEX; MAX_CHILDREN]
            };

            self.directories.dirs[cur_dir.index].child_indexes[i] = DELETE_INDEX;

            print!(
                "\n[Ok] Directory \"{}\" removed",
                core::str::from_utf8(&dir_name.clone())
                    .unwrap()
                    .trim_matches('\0')
            );
        }
    }

    fn dir_tree(&mut self, cur_dir: Dir, space_count: usize) 
    {
        println!();
        
        for i in 0..cur_dir.child_count as usize 
        {
            let child_dir = self.directories.dirs[cur_dir.child_indexes[i]];

            for i in 0..space_count 
            {
                for j in 0..4 
                {
                    print!(" ");
                }
            }

            print!(
                "/{}",
                core::str::from_utf8(&child_dir.name)
                    .unwrap()
                    .trim_matches('\0')
            );

            self.dir_tree(child_dir, space_count + 1);
        }
    }
}