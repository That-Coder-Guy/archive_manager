use blake3::Hasher;
use std::fs::{self, DirEntry, Metadata, File};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{self, Write, BufReader, Read};
use std::str::FromStr;
use serde_json;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

type CachedFiles = Arc<Mutex<Vec<CachedFile>>>;
type CachedFolders = Arc<Mutex<Vec<CachedFolder>>>;

#[derive(Serialize, Deserialize, Debug)]
struct CachedFile {
    filename: String,
    hash: String,
    date_modified: u128,
    date_created: u128,
}

impl CachedFile {
    fn new(file_path: PathBuf) -> io::Result<Self> {

        // Ensure that the provided path leads to a file
        let metadata: Metadata = file_path.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Path was not a file"));
        }
 
        // Get file name
        let filename = file_path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "File path was invalid"))?
            .to_owned()
            .into_string()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "File name contained invalid UTF-8 characters"))?;


        // Get file data hash
        let mut reader: BufReader<File> = BufReader::new(File::open(file_path)?);
        let mut hasher: Hasher = Hasher::new();
    
        let mut buffer: [u8; 8192] = [0; 8192];
        loop {
            let bytes_read: usize = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
    
        let hash: String = hasher.finalize().to_hex().to_string();

        // Get last modified timestamp of file
        let date_modified: u128 = system_time_to_millis(metadata.modified()?);

        // Get date created of file
        let date_created: u128;
        if let Ok(created) = metadata.created() {
            date_created = system_time_to_millis(created);
        }
        else {
            // TODO: Add a more exaustive list of methods to obtain the creation date
            date_created = date_modified;
        }

        return Ok(CachedFile {
            filename: filename,
            hash: hash,
            date_modified: date_modified,
            date_created: date_created,
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CachedFolder {
    folder_name: String,
    date_modified: u128,
    files: Vec<CachedFile>,
    folders: Vec<CachedFolder>
}

impl CachedFolder {
    fn new(folder_path: PathBuf) -> io::Result<Self> {
        // Ensure that the provided path leads to a folder
        let metadata: Metadata = folder_path.metadata()?;
        if !metadata.is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Path was not a folder"));
        }

        // Get folder name
        let folder_name = folder_path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Folder path was invalid"))?
            .to_owned()
            .into_string()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Folder name contained invalid UTF-8 characters"))?;

        // Get last modified timestamp of folder
        let date_modified: u128 = system_time_to_millis(metadata.modified()?);
        let mut files: Vec<CachedFile> = Vec::new();
        let mut folders: Vec<CachedFolder> = Vec::new();
        
        // Cache child files, folders, and symlinks
        for result in fs::read_dir(&folder_path)? {
            let entry: DirEntry = result?;
            let entry_metadata: Metadata = entry.metadata()?;
            let entry_path: PathBuf = entry.path().to_path_buf();
            if entry_metadata.is_file() {
                files.push(CachedFile::new(entry_path)?);
            }
            else if entry_metadata.is_dir() {
                folders.push(CachedFolder::new(entry_path)?);
            }
            else if entry_metadata.is_symlink() {
                return Err(io::Error::new(io::ErrorKind::Other, "Ayo there's a symlink in this hoe"));
            }
        }

        return Ok(CachedFolder {
            folder_name: folder_name,
            date_modified: date_modified,
            files: files,
            folders: folders
        });
    }

    fn threaded_new(folder_path: PathBuf) -> io::Result<Self> {
        // Ensure that the provided path leads to a folder
        let metadata: Metadata = folder_path.metadata()?;
        if !metadata.is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Path was not a folder"));
        }

        // Get folder name
        let folder_name = folder_path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Folder path was invalid"))?
            .to_owned()
            .into_string()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Folder name contained invalid UTF-8 characters"))?;

        // Get last modified timestamp of folder
        let date_modified: u128 = system_time_to_millis(metadata.modified()?);

        // Thread-safe collections for results
        let files: CachedFiles = Arc::new(Mutex::new(Vec::new()));
        let folders: CachedFolders = Arc::new(Mutex::new(Vec::new()));
        
        // Cache child files, folders, and symlinks
        let entries: Vec<DirEntry> = fs::read_dir(&folder_path)?
            .filter_map(Result::ok)
            .collect();

        entries.into_par_iter().for_each(|entry| {
            if let Err(error) = parse_entry(entry, &files, &folders) {
                println!("{error}");
            }
        });

        return Ok(CachedFolder {
            folder_name: folder_name,
            date_modified: date_modified,
            files: Arc::try_unwrap(files).unwrap().into_inner().unwrap(),
            folders: Arc::try_unwrap(folders).unwrap().into_inner().unwrap(),
        });
    }
}

fn parse_entry(entry: DirEntry, files: &CachedFiles, folders: &CachedFolders) -> io::Result<()> {
    let entry_metadata = entry.metadata()?;
    let entry_path: PathBuf = entry.path().to_path_buf();

    // If entry is a file
    if entry_metadata.is_file() {
        files.lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to lock files"))?
            .push(CachedFile::new(entry_path)?);
    }

    // If entry is directory
    else if entry_metadata.is_dir() {
        folders.lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to lock folders"))?
            .push(CachedFolder::new(entry_path)?);
    }

    // If entry is a symlink
    else if entry_metadata.is_symlink() {
        return Err(io::Error::new(io::ErrorKind::Other, "Symlinks not supported"));
    }

    return Ok(());
}

fn system_time_to_millis(system_time: SystemTime) -> u128 {
    match system_time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            return duration.as_millis();
        }
        Err(_) => {
            return 0;
        }
    }
}

pub fn create_cache() {
    let Ok(path) = PathBuf::from_str("C:\\Users\\Owner\\Desktop\\Terabyte Backup");
    let start = Instant::now();
    let folder: Result<CachedFolder, io::Error> = CachedFolder::threaded_new(path);
    let duration = start.elapsed();
    println!("Time taken: {:?}", duration); 
    // Time taken without threading: 1015.0344617s
    // Time taken with threading: 996.7444468s

    match folder
    {
        Ok(cache) => {
            if let Ok(json_string) = serde_json::to_string_pretty(&cache)
            {
                let file_path = "cache.json";
                if let Ok(mut file) = File::create(file_path)
                {
                    let _ = file.write_all(json_string.as_bytes());
                }
            }
        }
        Err(error) => {
            println!("{error}");
        }
    }
}