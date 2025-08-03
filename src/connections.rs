use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, sync::{Arc, Mutex}, thread};
use crossterm::{self, event::{self, Event}};
use ratatui;
use crate::tui;

pub fn connect(port: &str, ip: &str, name: &str) {
    
    println!("Connecting TCP {ip} on port {port}");

    let mut socket = match TcpStream::connect(format!("{ip}:{port}")){
        Err(_) => panic!("Can't connect to the desired ip at the desired port"),
        Ok(a) => a
    };

    println!("Connected to: {}", socket.local_addr().unwrap());
    connshell(&mut socket, name);

}

pub fn host(port: &str, name: &str) {

    println!("Binding TCP on port: {port}");
    
    let list = match TcpListener::bind(format!("127.0.0.1:{port}")){
        Err(_) => panic!("Can't bind the listener to the port"),
        Ok(a) => a
    };

    let mut conn = match list.accept(){
        Err(_) => panic!("Can't accept connecton from outside"),
        Ok(a) => a
    };

    connshell(&mut conn.0, name);
}

fn connshell(socket: &mut TcpStream, username: &str){ 

    // getting and sending the name 
    let _ = socket.write(username.as_bytes());
    let mut bufname = [0u8; 1024];
    let _ = socket.read(&mut bufname);
    let name = String::from_utf8(bufname.to_vec()).unwrap();
    let name = name.trim_matches(char::from(0)).trim().to_string();
    let thread_name = name.clone();

    println!("Succesfully connected with: {}, '{}'", socket.local_addr().unwrap(), name);

    // creating the messagess vectors 
    let messagess = Arc::new(Mutex::new(Vec::<String>::new())); // vector of messagess "NAME> MESSAGE", its shared between threads
    let thread_messagess1 = Arc::clone(&messagess);
    let thread_messagess2 = Arc::clone(&messagess);

    let user_msg = Arc::new(Mutex::new(String::new()));
    let user_msgclone = Arc::clone(&user_msg);

    let thread_socket: TcpStream = socket.try_clone().unwrap();

    // launching the threads, one for the receiving and one for the rendering of the TUI
    let _ = thread::spawn(move || recvmsg( thread_socket, thread_messagess1));
    let _ = thread::spawn(move || tui::renderthread( thread_messagess2, user_msgclone, thread_name));

    // sending messagess loop
    loop {
        let user_msgcmp = user_msg.lock().unwrap().to_string();
        if let Event::Key(key) = event::read().unwrap() { // event handler for keystrokes
            match key.code {
                event::KeyCode::Esc => {
                    ratatui::restore();
                    break;
                },
                event::KeyCode::Char(c) => user_msg.lock().unwrap().push(c), // push char action
                event::KeyCode::Backspace => { user_msg.lock().unwrap().pop(); }, // delete action
                event::KeyCode::Enter => { // send action (delete the string with only spaces " ")
                    let avalable_chars = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"; // we use this chars variable to allow writing texts that begin with a space
                    if &user_msgcmp == "" || user_msgcmp.starts_with(" ") && !user_msgcmp.chars().any(|f| avalable_chars.contains(f)){
                        user_msg.lock().unwrap().clear();
                    }
                    else {
                        messagess.lock().unwrap().push(format!("{username}> {}", user_msgcmp));
                        let _ = socket.write(format!("{username}> {}", user_msgcmp).trim().as_bytes());
                        if user_msgcmp == "!clear" {
                            messagess.lock().unwrap().clear();
                        }
                        user_msg.lock().unwrap().clear();
                    }
                }
                _ => {}
            }
        }
    }

}

fn recvmsg(mut socket: TcpStream,  messagess: Arc<Mutex<Vec<String>>>){
    // receive the message loop
    loop {
        let mut buffer = [0u8; 1024];
        let _ = match socket.read(&mut buffer){
            Err(_) => break,
            Ok(0) => break,
            Ok(_) => {}
        };
        let reader = String::from_utf8_lossy(&buffer).to_string();
        let reader = reader.trim_matches(char::from(0)).trim();
        messagess.lock().unwrap().push(format!("{reader}"));
    }
    println!("connection closed");
}


