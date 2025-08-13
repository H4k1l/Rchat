// importing modules
use crate::tui;
use crate::file_management;

// importing libraries
use std::{
    sync::Arc,
    path::{
        Path,
        absolute
    },
    fs::{
        File,
        remove_file,
        remove_dir,
        create_dir,
        exists
    },
    io::{
        Read,
        Write
    },
    time::Duration
};

use tokio::{ // using tokio runtime for a better variables handling
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::{
        TcpListener, 
        TcpStream,
        tcp::{
            OwnedWriteHalf,
            OwnedReadHalf
        }
    },
    sync::{Mutex},
};

use crossterm::{
    self, 
    event::{
        self, 
        Event,
        KeyModifiers
    }
};

use rpassword;
use ratatui; // ratatui for restoring the tui


// enum for managing events
#[derive(Clone, PartialEq, Debug)]
pub enum ConnEvent {
    ReceiveFile,
    SendFile,
    None
}   

pub async fn connect(ip: &str, port: &str, name: &str) {
    
    let mut socket = match TcpStream::connect(format!("{ip}:{port}")).await {
        Err(_) => panic!("Can't connect to the desired ip at the desired port"),
        Ok(a) => a
    };

    // AUTENTICATION PROCESS...
    let mut buffer = [0u8; 1024];
    let n = socket.read(&mut buffer).await.unwrap_or(0);
    let msg = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
    if msg == "!authcheck" {
        loop {
            println!("Enter the password for the connection: ");
            let password_str = rpassword::read_password().unwrap();
            let _ = socket.write_all(password_str.as_bytes()).await;

            let mut buffer = [0u8; 1024];
            let n = socket.read(&mut buffer).await.unwrap_or(0);
            let resp = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
            if resp == "!authpass" {
                let _ = socket.write_all(b"!authpass").await;
                break;
            }
            else {
                println!("You failed the autentication, retry...");
            }
        }
    }

    connshell(socket, name).await;
}

pub async fn host(port: &str, name: &str, password: bool) {

    let mut password_str = String::new();

    if password { // if password, get the password
        println!("Enter the password for the connection: ");
        password_str = rpassword::read_password().unwrap();
    }

    println!("awaiting for an external connection ...");

    let listener = match TcpListener::bind(format!("127.0.0.1:{port}")).await {
        Err(_) => panic!("Can't bind the listener to the port"),
        Ok(a) => a
    };

    let mut conn_stream = match listener.accept().await {
        Err(_) => panic!("Can't accept connecton from outside"),
        Ok(a) => a
    };
    
    // AUTENTICATION PROCESS...
    if password {
        let _ = conn_stream.0.write_all(b"!authcheck").await;
        loop {
            let mut buffer = [0u8; 1024];
            let n = conn_stream.0.read(&mut buffer).await.unwrap_or(0);
            let connpasswd = String::from_utf8_lossy(&buffer[..n]).trim().to_string();

            if connpasswd == password_str {
                let _ = conn_stream.0.write_all(b"!authpass").await;
                loop {
                    let mut buffer = [0u8; 1024];
                    let n = conn_stream.0.read(&mut buffer).await.unwrap_or(0);
                    let msg = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    if msg == "!authpass" {
                        break;
                    }
                }
                break;
            }
            else {
                let _ = conn_stream.0.write_all(b"!authfail").await;
            }
        }
    }
    else {
        let _ = conn_stream.0.write_all(b"!authpass").await;
    }
    std::thread::sleep(Duration::from_millis(100));

    connshell(conn_stream.0, name).await;
}

async fn connshell(mut socket: TcpStream, username: &str) {
    
    // getting and sending the user name
    let _ = socket.write(username.as_bytes()).await;
    let mut buffer = [0u8; 1024];
    let _ = socket.read(&mut buffer).await;
    let name = String::from_utf8(buffer.to_vec()).unwrap(); // doing trim_matches removes all the '\0' buffer-padding
    let name = name.trim_matches(char::from(0)).trim();

    // dividing the socket in reader and writer
    let (reader, mut writer) = socket.into_split();
    
    // creating the messages variable
    let messages = Arc::new(Mutex::new(Vec::<String>::new()));
    let thread_messages1 = Arc::clone(&messages);
    let thread_messages2 = Arc::clone(&messages);

    // creating other shared Arc variables (Arc types are from std, Mutex types are from tokio)
    let user_msg = Arc::new(Mutex::new(String::new()));
    let thread_user_msg = Arc::clone(&user_msg);
    //
    let conname = Arc::new(name.to_string());
    let thread_conname1 = Arc::clone(&conname);
    //
    let scroll = Arc::new(Mutex::new(0 as u16));
    let thread_scroll = Arc::clone(&scroll);
    //
    let view_height = Arc::new(Mutex::new(0 as u16));
    let thread_view_height = Arc::clone(&view_height);
    //
    let event = Arc::new(Mutex::new(ConnEvent::None));
    let thread_event1 = Arc::clone(&event);
    let thread_event2 = Arc::clone(&event);

    // receive the message
    let _ = tokio::spawn(async move { recvmsg(reader, thread_messages1, thread_conname1, thread_event1).await });
    // render the tui
    let _ = tokio::spawn( async move { tui::renderthread(thread_messages2, thread_user_msg, thread_scroll, thread_view_height, conname.to_string(), thread_event2).await });

    loop {
        let user_msgcmp = user_msg.lock().await.to_string();
        let scrollcmp = scroll.lock().await.clone();
        let view_heightcmp = view_height.lock().await.clone();
        
        if let Event::Key(key) = event::read().unwrap() { // scanning the actions of the keystrokes
            match key.code {
                event::KeyCode::Char(c) => user_msg.lock().await.push(c),
                event::KeyCode::Backspace => {user_msg.lock().await.pop();},
                event::KeyCode::Enter => {// send action (delete the string with only spaces " ")
                    let avalable_chars = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"; // we use this chars variable to allow writing texts that begin with a space
                    if *event.lock().await == ConnEvent::None && &user_msgcmp == "" || user_msgcmp.starts_with(" ") && !user_msgcmp.chars().any(|f| avalable_chars.contains(f)){
                        user_msg.lock().await.clear();
                        let _ = avalable_chars;
                        continue;
                    }
                    let _ = avalable_chars; // dropping the value
                    if user_msgcmp.starts_with("!") { // see if there's any command
                        match user_msgcmp.as_str() {
                            "!clear" => {
                                messages.lock().await.clear();
                                *scroll.lock().await = 0;
                            },
                            "!sendfile" => {
                                *event.lock().await = ConnEvent::SendFile;
                                user_msg.lock().await.clear();
                                continue;
                            },
                            "!cancel" | "!cl" => {
                                *event.lock().await = ConnEvent::None;
                            }
                            _ => { 
                                let msg = format!("!COMMAND '{user_msgcmp}' NOT FOUND!");
                                messages.lock().await.push(msg);
                            } 
                        }
                    } 
                    eventhandler(&mut writer, Arc::clone(&event), Arc::clone(&messages), Arc::clone(&scroll), Arc::clone(&user_msg), username).await;
                    user_msg.lock().await.clear();
                },
                event::KeyCode::Up => {
                    if *event.lock().await == ConnEvent::ReceiveFile {
                        *scroll.lock().await = 0;
                    }
                    else if scrollcmp > 0 {
                        *scroll.lock().await -= 1;
                    }
                },
                event::KeyCode::Down => {
                    if *event.lock().await == ConnEvent::ReceiveFile {
                        *scroll.lock().await = 1;
                    }
                    else if scrollcmp + view_heightcmp / 3 < (messages.lock().await.len() as u16) {
                        *scroll.lock().await += 1;
                    }
                },
                _ => {}
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == event::KeyCode::Char('c') { // check ctrl-c
                ratatui::restore();
                std::process::exit(0);
            }
        }
    }

}

// this function handle the events
async fn eventhandler(writer: &mut OwnedWriteHalf, event: Arc<Mutex<ConnEvent>>, messages: Arc<Mutex<Vec<String>>>, selection: Arc<Mutex<u16>>, user_msg: Arc<Mutex<String>>, username: &str) {
    let event_cmp = event.lock().await.clone();
    let user_msgcmp = user_msg.lock().await.clone();
    match event_cmp  {
        ConnEvent::None => { // default behaviour, push the message in the messages vector
            if !user_msgcmp.starts_with("!") {
                messages.lock().await.push(format!("{username}> {user_msgcmp}"));
                let _ = writer.write(user_msgcmp.as_bytes()).await;
            };
        },
        ConnEvent::ReceiveFile => { // receive the file behaviour
            if *selection.lock().await == 0 {
                let _ = writer.write(b"y").await;
                messages.lock().await.push(format!("YOU ACCEPTED THE FILE!"));
            }
            else if *selection.lock().await == 1 {
                let _ = writer.write(b"n").await;
                messages.lock().await.push(format!("!YOU HAVEN'T ACCEPTED THE FILE!"));
            }
            *event.lock().await = ConnEvent::None;
        },
        ConnEvent::SendFile => { // send the file behaviour
            let path = Path::new(&user_msgcmp);
            if !path.exists() {
                messages.lock().await.push(format!("!FILE '{user_msgcmp}' NOT FOUND!"));
            }
            else { // checking if user want file
                file_management::preparezip(path);
               messages.lock().await.push(format!("AWAITING FOR THE USER RESPONSE!"));
                user_msg.lock().await.clear();
                let _ = writer.write(b"!sendfile").await;
                *event.lock().await = ConnEvent::None;         
                loop { // wait the other user response
                    if messages.lock().await.last().unwrap().split_once(">").unwrap_or(("None", "None")).1 == " y" {
                        messages.lock().await.pop();
                        messages.lock().await.push("THE USER ACCEPTED THE FILE!".to_string());
                        sendfile(writer, messages, format!("processing/{}", path.file_name().unwrap().to_str().unwrap().to_string())).await;
                        break;
                    }
                    else if messages.lock().await.last().unwrap().split_once(">").unwrap_or(("None", "None")).1 == " n" {
                        messages.lock().await.pop();
                        messages.lock().await.push(format!("!THE USER REJECT THE FILE {}!", user_msgcmp));
                        break;
                    }
                }
            }
        }
    }
}

// this function work on a separate thread to receive all the messages
async fn recvmsg(mut reader: OwnedReadHalf, messages: Arc<Mutex<Vec<String>>>, conname: Arc<String>, event: Arc<Mutex<ConnEvent>>) {
    loop {
        
        let mut buffer = [0; 1024];
        
        let _ = match reader.read(&mut buffer).await {
            Err(_) => break,
            Ok(0) => break,
            Ok(_) => {}
        };


        let mut msg = String::from_utf8_lossy(&buffer).to_string();
        msg = msg.trim_matches(char::from(0)).to_string();

        if msg == "!sendfile" {
            *event.lock().await = ConnEvent::ReceiveFile;
            recvfile(&mut reader, Arc::clone(&messages)).await;
            continue;
        }

        messages.lock().await.push(format!("{conname}> {msg}"));
    }
}

async fn recvfile(reader: &mut OwnedReadHalf, messages: Arc<Mutex<Vec<String>>>) { // receiving the file function
    loop {
        if messages.lock().await.last().unwrap_or(&"None".to_string()) == &"YOU ACCEPTED THE FILE!".to_string() { // check if the log is emitted
            let mut buffer = [0u8; 1024];
            let _ = reader.read(&mut buffer).await;
            if !exists("received").unwrap() {
                create_dir("received").unwrap();
            }
            let syncmsg = String::from_utf8_lossy(&buffer);
            let syncmsg = syncmsg.trim_matches(char::from(0));
            let filename = syncmsg.replace("!sending ", "");
            let path = absolute(Path::new("received").join(filename.clone())).unwrap();
            if syncmsg.starts_with("!sending") { // syncronize
                let mut vecbytes: Vec<u8> = Vec::new(); // getting the file
                loop {
                    let mut bytes = [0; 4096];
                    match reader.read(&mut bytes).await {
                        Err(_) => break,
                        Ok(0) => break,
                        Ok(a) => {
                            if String::from_utf8_lossy(&bytes).trim_matches(char::from(0)).trim() == "!end" {
                                break;
                            }
                            vecbytes.extend(&bytes[..a]);
                        }
                    }
                }
                let mut file = File::create(path).unwrap();                
                let _ = match file.write_all(&vecbytes) {
                    Ok(_) => messages.lock().await.push(format!("FILE SAVED AS 'received/{}'", &filename.trim())),
                    Err(_) => break
                };
                file_management::extract_zip(&format!("received/{}", filename.trim()));
                let _ = remove_file(&format!("received/{}", filename.trim()));
                messages.lock().await.push(format!("FINISHED EXTRACTING"));
            }
        }
    }
}

async fn sendfile(writer: &mut OwnedWriteHalf, messages: Arc<Mutex<Vec<String>>>, filename: String) { // send file function
    let filename = format!("{}.zip",filename.split("/").last().unwrap());
    loop {
        if messages.lock().await.last().unwrap_or(&"None".to_string()) == &"THE USER ACCEPTED THE FILE!".to_string() { // check if the log is emitted
            let _ = writer.write(format!("!sending {}", &filename).as_bytes()).await;
            std::thread::sleep(std::time::Duration::from_millis(100));
            break;
        }
    }
    let mut zipfile = File::open(&filename).unwrap(); // getting the file
    let mut zipcontent: Vec<u8> = Vec::new();
    zipfile.read_to_end(&mut zipcontent).unwrap();

    let _ = writer.write(&zipcontent).await; // sending the file
    std::thread::sleep(std::time::Duration::from_millis(100));

    // for chunk in zipcontent.chunks(8000000) { // 1 mbs  // THIS METOD DIVIDE IN CHUNKS THE FILE
    //     messages.lock().await.push(format!("Sending... bytes {}", chunk.len()));
    //     let _ = writer.write(chunk).await;
    //     std::thread::sleep(std::time::Duration::from_millis(100));
    // }

    let _ = writer.write("!end".as_bytes()).await;
    messages.lock().await.push("FILE SENDED SUCCESFULLY!".to_string());
    if Path::new(&filename).is_file() {
        let _ = remove_file(filename);
    }
    else {
        let _ = remove_dir(filename);
    }
}
