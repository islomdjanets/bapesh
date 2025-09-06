use std::{
    fs::{self, OpenOptions},
    io::{Read, Result, Write, SeekFrom, Seek}, path::Path,
};

use crate::json::JSON;

pub struct Driver;

impl Driver {
    pub fn read_binary(path: &str) -> Result<Vec<u8>> {
        fs::read(path)
    }

    pub fn read_to_string(path: &str) -> Result<String> {
        fs::read_to_string(path)
    }

    pub fn is_directory( path: &str) -> bool {
        Path::new(path).is_dir()
    }

    pub fn read(path: &str) -> Result<Vec<u8>> {
        //fs::read_to_string(path)
        fs::read(path)
        //fs::re
    }

    pub fn read_n( file: fs::File, n: usize ) -> Vec<u8> {
        //let mut list_of_chunks = Vec::new();

        //let chunk_size = 0x4000;

        //loop {
            let mut chunk = Vec::with_capacity(n);
            let n = file.take(n as u64).read_to_end(&mut chunk);
            //if n == 0 { break; }
            //list_of_chunks.push(chunk);
            //if n < chunk_size { break; }
        //}

        chunk
    }

    pub fn read_json( path: &str) -> Option<JSON> { // remove!!!
        //let file = Driver::create_file("data.json".into(), content).unwrap();
        let data = Driver::read_to_string(path);
        if data.is_err() {
            return None;
            // return Err(data);
            // return serde_json::Error {};
        }

        let content = serde_json::from_str(&data.unwrap());
        if let Ok(content) = content {
            return content;
        };
        None
            //.expect("Can't parse json"); // convert string to json object and panic in case of error
    }

    pub fn read_from_to( mut file: fs::File, from: usize, to: usize ) -> Result<Vec<u8>> {
        const OFFSET: u64 = 4096;
        const READ_SIZE: usize = 1024;
        let mut buf = [0u8; READ_SIZE];
        //let mut file = File::open("some_file.bin");
        file.seek(SeekFrom::Start(OFFSET));
        
        match file.read_exact(&mut buf) {
            Ok(()) => Ok(buf.to_vec()),
            Err(error) => Err(error)// println!("Error reading file")
        } //.expect()

        //Ok(buf.to_vec())
    }

    pub fn read_from( mut file: fs::File, from: u64, amount: u64 ) -> Result<Vec<u8>> {
        //let OFFSET: usize = from;
        const READ_SIZE: usize = 1024;
        let mut buf = [0u8; READ_SIZE];
        file.seek(SeekFrom::Start(from));

        file.read_exact(&mut buf); //.expect("Error reading file")

        Ok(buf.to_vec())
    }

    pub fn write( path: String, content: String ) -> Result<()> {
        std::fs::write(path, content)
    }

    pub fn is_binary(content_type: &String) -> bool {
        content_type == "image/webp" || content_type == "image/jpeg" || content_type == "image/png"
    }

    pub fn create_file(path: String, content: String) -> Result<fs::File> {
        let mut file = fs::File::create(path).unwrap();
        if !content.is_empty() {
            file.write_all(content.as_bytes()).unwrap();
        }

        //fs::write("myfile.txt", "new Data");
        //println!("{:?}", file);
        Ok(file)
    }

    pub fn create_directory(path: String, open: bool) -> Option<fs::ReadDir> {
        match fs::create_dir(&path) {
            Ok(_) => {
                if open {
                    return Some(fs::read_dir(&path).unwrap());
                }
            },
            Err(_) => return None,
        };
        None
    }

    pub fn open_directory(path: String, create: bool) -> Option<fs::ReadDir> {
        let result = fs::read_dir(&path);
        if let Ok(dir) = result {
            return Some(dir);
        }

        if create {
            let dir = Driver::create_directory(path, true);
            return dir;
            // return Some(dir);
            // if let Some(dir) = dir {
            //     return dir;
            // }
        }

        None
    }

    pub fn open_file(path: String) -> Result<fs::File> {

        fs::File::open(path)

        // let mut content = String::new();

        // match fs::File::open(path) {
        //     Ok(mut file) => {
        //         file.read_to_string(&mut content).unwrap();
        //         Some(content)
        //     }
        //     Err(_) => None,
        // }
    }

    pub fn open_with_options(
        path: &str,
        create: bool,
        append: bool,
        read: bool,
        write: bool,
    ) -> Result<fs::File> {
        OpenOptions::new()
            .create(create)
            .append(append)
            .read(read)
            .write(write)
            .open(path)
    }

    pub fn erase(path: &str) -> bool {
        fs::remove_file(path).is_ok()
    }

    pub fn delete_folder(path: &str) -> bool {
        fs::remove_dir_all(path).is_ok()
    }
}
