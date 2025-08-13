use std::{
    fs::{
        self, 
        File
    }, 
    io::{
        self, 
        Read, 
        Write
    }, 
    path::Path
};
use zip::{
    write::FileOptions, 
    ZipWriter, 
    CompressionMethod
};

pub fn preparezip(path: &Path) -> bool{ // given a file, prepare the zip 
    if path.exists() {
        let file = File::create(format!("{}.zip", path.file_name().unwrap().to_str().unwrap())).unwrap();
        let mut zip = ZipWriter::new(file); 
        if path.is_file() { // if file 
            let dst = format!("./{}", path.file_name().unwrap().display());
            let _ = fs::copy(&path, &dst);
            add_file_in_zip(&mut zip, &dst);

        }
        else { // else -> dir 
            let dirname = format!("./{}", path.iter().last().unwrap().to_str().unwrap());
            let mut dst = Path::new(&dirname);
            copy_dir(&mut path.to_str().unwrap().to_string(), &mut dst);
            add_dir_in_zip(&mut zip, path.to_str().unwrap(), path.parent().unwrap().to_str().unwrap());
        }
        zip.finish().unwrap();
        return true;
    }
    false
}

pub fn extract_zip(fname: &String) { // extract the dir in the "./received" directory
    let file = fs::File::open(fname).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() { // extracting for every item in the zip
        let mut file = archive.by_index(i).unwrap();
        let mut outpath = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };
        outpath = Path::new("received").join(outpath);
        if file.is_dir() { // if dir, create the dir, else create the file
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
}

fn add_dir_in_zip(zip: &mut ZipWriter<File>, path: &str, basepath: &str) { // zip a directory 
    let options: FileOptions<'_, ()> = FileOptions::default().compression_method(CompressionMethod::Stored);
    let path = Path::new(&path);

    for file in fs::read_dir(&path).expect("err: Can't read dir '{path}'"){
        let file = file.unwrap();
        if file.file_type().unwrap().is_dir(){
            let newpath: String = file.path().to_str().unwrap().to_string();
            add_dir_in_zip(zip, &newpath, basepath);
        }
        else if file.file_type().unwrap().is_file(){
            let newpath: String = file.path().to_str().unwrap().to_string();
            let relpath = file.path().strip_prefix(basepath).unwrap().to_str().unwrap().to_string();
            zip.start_file(relpath, options).expect("err: Can't create file in zip");
            let mut content: Vec<u8> = Vec::new();
            let _ = File::open(newpath).unwrap().read_to_end(&mut content);
            zip.write_all(&content).expect("err: Can't write in zip");
        }
    }
}

fn add_file_in_zip(zip: &mut ZipWriter<File>, input_path: &str) { // zip a file
    let mut input_file = File::open(input_path).unwrap();

    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer).unwrap();

    zip.start_file(input_path, options).expect("err: Can't create file in zip");
    zip.write_all(&buffer).expect("err: Can't write in zip");
}

fn copy_dir(src: &mut String, dst: &mut &Path) { // copy dir is used for sending the file, copying the file in the "./processing" directory
    fs::create_dir_all(&dst).expect("err: Can't create dir '{dst}'");
    for entry in fs::read_dir(&src).expect("err: Can't read '{src}'"){
        let entry = entry.unwrap();
        let datatype = entry.file_type().unwrap();
        let nwdst = *dst;
        if datatype.is_dir(){
            let mut nwsrc =  entry.path().to_str().unwrap().to_string();
            copy_dir(&mut nwsrc, &mut nwdst.join(entry.file_name()).as_path());
        }
        else {
            fs::copy(entry.path(), &mut nwdst.join(entry.file_name()).as_path()).expect("err: Can't copy file in '{dst}'");
        }
    }
}
