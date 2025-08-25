// importing modules
use crate::tui;
use crate::file_management;
use crate::encryption;

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
use rpassword; // rpassword for password protected chats
use aes_gcm::{Aes256Gcm, aead::Nonce}; // importing this library for creating the nonce
use ratatui; // ratatui for restoring the tui


// enum for managing events
#[derive(Clone, PartialEq, Debug)]
pub enum ConnEvent {
    ReceiveFile,
    SendFile,
    None
}   

pub async fn connect(ip: &str, port: &str, name: &str) {
    
    // CONNECTION ACCEPTING PROCESS...

    let mut socket = match TcpStream::connect(format!("{ip}:{port}")).await {
        Err(_) => panic!("Can't connect to the desired ip at the desired port"),
        Ok(a) => a
    };

    // ENCRYPTION PROCESS...
    encryption::gen_ecc_keys().await; 
    let nonce = encryption::gen_nonce().await;

    // keys exchange
    encryption::get_remote_ecc_key(&mut socket).await;
    encryption::send_remote_ecc_key(&mut socket).await;

    let mut keybuffer = [0u8; 1024];
    let n = socket.read(&mut keybuffer).await.unwrap();
    let keybuffer = keybuffer[..n].to_vec();

    let key = encryption::ecc_dec(keybuffer).await;

    // nonce exchange
    let mut noncebuffer = [0u8; 1024];
    let n = socket.read(&mut noncebuffer).await.unwrap();
    let noncebuffer = noncebuffer[..n].to_vec();

    let remote_nonce = encryption::ecc_dec(noncebuffer).await;
    let remote_nonce = Nonce::<Aes256Gcm>::clone_from_slice(&remote_nonce);
    
    let enc_nonce = encryption::ecc_enc(nonce.clone().to_vec()).await;
    let _ = socket.write(&enc_nonce).await;

    // splitting the socket for an easier use
    let (mut reader, mut writer) = socket.into_split();

    println!("enstablished connection!");

    // AUTENTICATION PROCESS...
   let msg = recvenc(&mut reader, key.clone(), remote_nonce).await;
   let msg = String::from_utf8_lossy(&msg).trim().to_string();

    
    if msg == "!authcheck" {
        loop {
            println!("Enter the password for the connection: ");
            let password_str = rpassword::read_password().unwrap();
            sendenc(&mut writer, password_str.as_bytes().to_vec(), key.clone(), nonce).await;

            let resp = recvenc(&mut reader, key.clone(), remote_nonce).await;
            let resp = String::from_utf8_lossy(&resp).trim().to_string();

            if resp == "!authpass" {
                sendenc(&mut writer, "!authpass".to_string().as_bytes().to_vec(), key.clone(), nonce).await;
                break;
            }
            else {
                println!("You failed the autentication, retry...");
            }
        }
    }
    else if msg.len() == 0{
        println!("getted nothing!")
    }

    connshell(reader, writer, name, key, nonce, remote_nonce).await;

}

pub async fn host(port: &str, name: &str, password: bool) {

    // CONNECTION ACCEPTING PROCESS...
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

    // ENCRYPTION PROCESS... 
    encryption::gen_ecc_keys().await; 
    let nonce = encryption::gen_nonce().await;

    // keys exchange
    encryption::send_remote_ecc_key(&mut conn_stream.0).await;
    encryption::get_remote_ecc_key(&mut conn_stream.0).await;

    let key = encryption::gen_aes_key().await;
    let enc_key = encryption::ecc_enc(key.to_vec()).await;
    let _ = conn_stream.0.write(&enc_key).await;

    // nonce exchange
    let enc_nonce = encryption::ecc_enc(nonce.clone().to_vec()).await;
    let _ = conn_stream.0.write(&enc_nonce).await;

    let mut noncebuffer = [0u8; 1024];
    let n = conn_stream.0.read(&mut noncebuffer).await.unwrap();
    let noncebuffer = noncebuffer[..n].to_vec();

    let remote_nonce = encryption::ecc_dec(noncebuffer).await;
    let remote_nonce = Nonce::<Aes256Gcm>::clone_from_slice(&remote_nonce);

    // splitting the socket for an easier use
    let (mut reader, mut writer) = conn_stream.0.into_split();

    println!("connection enstablished!");
    
    // AUTENTICATION PROCESS...
    if password {
        sendenc(&mut writer, "!authcheck".to_string().as_bytes().to_vec(), key.clone(), nonce).await;

        loop {
            let connpasswd = recvenc(&mut reader, key.clone(), remote_nonce).await;
            let connpasswd = String::from_utf8_lossy(&connpasswd).trim().to_string();

            if connpasswd == password_str {
                sendenc(&mut writer, "!authpass".to_string().as_bytes().to_vec(), key.clone(), nonce).await;
                loop {
                    let msg = recvenc(&mut reader, key.clone(), remote_nonce).await;
                    let msg = String::from_utf8_lossy(&msg).trim().to_string();
                    if msg == "!authpass" {
                        break;
                    }
                }
                break;
            }
            else {
                sendenc(&mut writer, "!authfail".to_string().as_bytes().to_vec(), key.clone(), nonce).await;
            }
        }
    }
    else {
        sendenc(&mut writer, "!authpass".to_string().as_bytes().to_vec(), key.clone(), nonce).await;
    }
    std::thread::sleep(Duration::from_millis(100));

    connshell(reader, writer, name, key, nonce, remote_nonce).await;

}

async fn connshell(mut reader: OwnedReadHalf, mut writer: OwnedWriteHalf, username: &str, session_key: Vec<u8>, nonce: Nonce::<Aes256Gcm>, remote_nonce: Nonce<Aes256Gcm>) {
    
    // getting and sending the user name
    sendenc(&mut writer, username.to_string().as_bytes().to_vec(), session_key.clone(), nonce).await;
    let name = recvenc(&mut reader, session_key.clone(), remote_nonce).await;
    let name = String::from_utf8_lossy(&name).to_string();

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
    //
    let session_key = Arc::new(session_key);
    let thread_session_key = Arc::clone(&session_key);

    // receive the message
    let _ = tokio::spawn(async move { recvmsg(reader, thread_messages1, thread_conname1, thread_event1, (*thread_session_key).clone(), remote_nonce).await });
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
                    eventhandler(&mut writer, Arc::clone(&event), Arc::clone(&messages), Arc::clone(&scroll), Arc::clone(&user_msg), username, (*session_key).clone(), nonce).await;
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
async fn eventhandler(writer: &mut OwnedWriteHalf, event: Arc<Mutex<ConnEvent>>, messages: Arc<Mutex<Vec<String>>>, selection: Arc<Mutex<u16>>, user_msg: Arc<Mutex<String>>, username: &str, session_key: Vec<u8>, nonce: Nonce<Aes256Gcm>) {
    let event_cmp = event.lock().await.clone();
    let user_msgcmp = user_msg.lock().await.clone();

    match event_cmp  {
        ConnEvent::None => { // default behaviour, push the message in the messages vector
            if !user_msgcmp.starts_with("!") {
                messages.lock().await.push(format!("{username}> {user_msgcmp}"));
                sendenc(writer, user_msgcmp.as_bytes().to_vec(), session_key.clone(), nonce).await;
            };
        },
        ConnEvent::ReceiveFile => { // receive the file behaviour
            if *selection.lock().await == 0 {
                sendenc(writer, "y".to_string().as_bytes().to_vec(), session_key.clone(), nonce).await;
                messages.lock().await.push(format!("YOU ACCEPTED THE FILE!"));
            }
            else if *selection.lock().await == 1 {
                sendenc(writer, "n".to_string().as_bytes().to_vec(), session_key.clone(), nonce).await;
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

                sendenc(writer, "!sendfile".to_string().as_bytes().to_vec(), session_key.clone(), nonce).await;
                *event.lock().await = ConnEvent::None;         
                loop { // wait the other user response
                    if messages.lock().await.last().unwrap().split_once(">").unwrap_or(("None", "None")).1 == " y" {
                        messages.lock().await.pop();
                        messages.lock().await.push("THE USER ACCEPTED THE FILE!".to_string());
                        sendfile(writer, messages, format!("processing/{}", path.file_name().unwrap().to_str().unwrap().to_string()), session_key.clone(), nonce).await;
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
async fn recvmsg(mut reader: OwnedReadHalf, messages: Arc<Mutex<Vec<String>>>, conname: Arc<String>, event: Arc<Mutex<ConnEvent>>, session_key: Vec<u8>, remote_nonce: Nonce<Aes256Gcm>) {
    loop {
        
        let mut buffer = [0; 1024];
        
        let n = match reader.read(&mut buffer).await {
            Err(_) => break,
            Ok(0) => break,
            Ok(n) => n
        };

        let buffer = buffer[..n].to_vec();

        let msg = encryption::aes_dec(buffer, session_key.clone(), remote_nonce).await;
        let msg = String::from_utf8_lossy(&msg).to_string();

        if msg == "!sendfile" {
            *event.lock().await = ConnEvent::ReceiveFile;
            recvfile(&mut reader, Arc::clone(&messages), session_key.clone(), remote_nonce).await;
            *event.lock().await = ConnEvent::None;
            continue;
        }

        messages.lock().await.push(format!("{conname}> {msg}"));
    }
}

async fn recvfile(reader: &mut OwnedReadHalf, messages: Arc<Mutex<Vec<String>>>, session_key: Vec<u8>, remote_nonce: Nonce<Aes256Gcm>) { // receiving the file function
    loop { 
        if messages.lock().await.last().unwrap_or(&"None".to_string()) == &"YOU ACCEPTED THE FILE!".to_string() { // check if the log is emitted
       
            if !exists("received").unwrap() {
                create_dir("received").unwrap();
            }
            let msg: Vec<u8> = recvenc(reader, session_key.clone(), remote_nonce).await;
            let syncmsg = String::from_utf8_lossy(&msg);
            let syncmsg = syncmsg.trim_matches(char::from(0));
        
            let infoiter = syncmsg.split(" ").collect::<Vec<&str>>();
            let filename = infoiter[1];
            let filelength = infoiter[2].parse::<usize>().unwrap();
            messages.lock().await.push(format!("the file length is: {} bytes", filelength));
        
            let path = absolute(Path::new("received").join(filename)).unwrap();
            if syncmsg.starts_with("!sending") { // syncronize
                let mut vecbytes: Vec<u8> = Vec::with_capacity(filelength); 
                while vecbytes.len() < filelength { // getting the file
                    let mut bytes = [0; 100000];
                    let n = reader.read(&mut bytes).await.unwrap();
                    vecbytes.extend_from_slice(&bytes[..n]);
                } 
                let vecbytes = encryption::aes_dec(vecbytes.to_vec(), session_key.clone(), remote_nonce).await; // decrypt the file

                let mut file = File::create(path).unwrap();                
                let _ = match file.write_all(&vecbytes) {
                    Ok(_) => messages.lock().await.push(format!("FILE SAVED AS 'received/{}'", &filename.trim())),
                    Err(_) => break
                };
                file_management::extract_zip(&format!("received/{}", filename.trim()));
                let _ = remove_file(&format!("received/{}", filename.trim()));
                messages.lock().await.push(format!("FINISHED EXTRACTING"));
                break;
            }
        }
    }
}

async fn sendfile(writer: &mut OwnedWriteHalf, messages: Arc<Mutex<Vec<String>>>, filename: String, session_key: Vec<u8>, nonce: Nonce::<Aes256Gcm>) { // send file function
    let filename = format!("{}.zip",filename.split("/").last().unwrap());
    loop {
        if messages.lock().await.last().unwrap_or(&"None".to_string()) == &"THE USER ACCEPTED THE FILE!".to_string() { // check if the log is emitted
            break;
        }
    }
    
    let mut zipfile = File::open(&filename).unwrap(); // getting the file
    let mut zipcontent: Vec<u8> = Vec::new();
    zipfile.read_to_end(&mut zipcontent).unwrap();
    let enczipfile = encryption::aes_enc(zipcontent, session_key.clone(), nonce).await; // encrypt the file

    sendenc(writer, format!("!sending {} {}", &filename, enczipfile.len()).as_bytes().to_vec(), session_key.clone(), nonce).await; // sending the name and the length of the file
    std::thread::sleep(std::time::Duration::from_millis(100));


    messages.lock().await.push(format!("the file length is: {} bytes", enczipfile.len()));
    for chunk in enczipfile.chunks(100000) { // dividing the file in chunks
        let _ = writer.write(&chunk).await; // sending the file
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    messages.lock().await.push("FILE SENDED SUCCESFULLY!".to_string());
    if Path::new(&filename).is_file() {
        let _ = remove_file(filename);
    }
    else {
        let _ = remove_dir(filename);
    }
}

async fn sendenc(writer: &mut OwnedWriteHalf, user_msg: Vec<u8>, session_key: Vec<u8>, nonce: Nonce<Aes256Gcm>) {

    let enc_msg = encryption::aes_enc(user_msg.clone(), session_key.clone(), nonce).await;
    let _ = writer.write(&enc_msg).await;

}

async fn recvenc(reader: &mut OwnedReadHalf, session_key: Vec<u8>, remote_nonce: Nonce<Aes256Gcm>) -> Vec<u8> {
    
    let mut buffer = [0u8; 1024];
    let n = reader.read(&mut buffer).await.unwrap();
    let buffer = buffer[..n].to_vec();

    encryption::aes_dec(buffer, session_key, remote_nonce).await

}