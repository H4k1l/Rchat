// importing libraries
use std::{
    io::{
        Write
    },
    fs::{
        remove_dir,
        create_dir,
        read_to_string, 
        File
    },
};
use aes_gcm::{
    Aes256Gcm, 
    KeyInit, 
    aead::{
        Aead, 
        Nonce
    }
};
use k256::{ // use the Secp256k1 eliptic curve for asymmetric encryption
    PublicKey, 
    SecretKey
};
use ecies::{ // using "ecies" for encrypt bytes with the Secp256k1 eliptic curve
    encrypt, 
    decrypt
};  
use rand::{
    RngCore, 
    rngs::OsRng
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream
};

// simmetric encryption functions

pub async fn gen_aes_key() -> Vec<u8> { // generate the Aes key 

    let mut keybuff = [0u8; 32];
    let _ = rand::rngs::OsRng.try_fill_bytes(&mut keybuff);
    
    aes_gcm::Key::<Aes256Gcm>::from_mut_slice(&mut keybuff).to_vec()
    
}

pub async fn aes_enc(plaintext: Vec<u8>, mut key: Vec<u8>, nonce: Nonce<Aes256Gcm>) -> Vec<u8> { // Aes encrypt function

    key.resize(32, 0);
    let key = aes_gcm::Key::<Aes256Gcm>::from_mut_slice(&mut key);

    let signature = ""; // no additional data required
    let payload = aes_gcm::aead::Payload {msg: &plaintext, aad: signature.as_bytes()}; 

    // generate cipher
    let cipher = Aes256Gcm::new(key);
    cipher.encrypt(&nonce, payload).unwrap() // bytes of the message

}

pub async fn aes_dec(ciphertext: Vec<u8>, mut key: Vec<u8>, nonce: Nonce<Aes256Gcm>) -> Vec<u8> { // Aes decrypt function

    key.resize(32, 0);
    let key = aes_gcm::Key::<Aes256Gcm>::from_mut_slice(&mut key);

    let signature = ""; // no additional data required
    let payload = aes_gcm::aead::Payload {msg: &ciphertext, aad: signature.as_bytes()}; 
    
    // generate cipher
    let cipher = Aes256Gcm::new(key);
    cipher.decrypt(&nonce, payload).unwrap() // bytes of the message

}

pub async fn gen_nonce() -> Nonce<Aes256Gcm> { // generate the nonce
    let mut noncebuff = [0u8; 12];
    let _ = rand::rngs::OsRng.try_fill_bytes(&mut noncebuff);
    Nonce::<Aes256Gcm>::clone_from_slice(&noncebuff)
}

// asymmetric encryption functions

pub async fn gen_ecc_keys() { // use of Secp256k1 for asymmetric encryption

    // remove all the old keys to create new ones
    let _ = remove_dir("keys");
    let _ = create_dir("keys"); 

    // generating the random secret key
    let seckey = SecretKey::random(&mut OsRng);
    let pubkey = seckey.public_key();

    // key encoding for better storage
    let encseckey = hex::encode(seckey.to_bytes());
    let encpubkey = hex::encode(pubkey.to_sec1_bytes());

    // saving the keys
    let mut seckeyfile = File::create("keys/lcalseckey.txt").unwrap();
    let _ = seckeyfile.write_all(encseckey.as_bytes());

    let mut pubkeyfile = File::create("keys/lcalpubkey.txt").unwrap();
    let _ = pubkeyfile.write_all(encpubkey.as_bytes());

}

pub async fn ecc_enc(plaintext: Vec<u8>) -> Vec<u8> { // Ecc encrypt function

    let publickey = load_remote_ecc_key();
    let enctext = encrypt(&publickey.to_sec1_bytes(), &plaintext).unwrap();

    enctext

}

pub async fn ecc_dec(ciphertext: Vec<u8>) -> Vec<u8> { // Ecc decrypt function

    let secretkey = load_local_ecc_keys().1;
    let dectext = decrypt(&secretkey.to_bytes(), &ciphertext).unwrap();

    dectext

}

pub async fn get_remote_ecc_key(socket: &mut TcpStream) { // receiving and saving the public key of the other host

    let mut keybuffer = [0u8; 1024];
    let n = socket.read(&mut keybuffer).await.unwrap();
    let keybuffer = keybuffer[..n].to_vec();

    let ecc_remote_pub_key = String::from_utf8(keybuffer.to_vec()).unwrap().trim_matches(char::from(0)).trim().to_string();
    let mut file = File::create("keys/remotepubkey.txt").unwrap();

    let _ = file.write_all(ecc_remote_pub_key.as_bytes());

}

pub async fn send_remote_ecc_key(socket: &mut TcpStream) { // sending the public key to the other host

    let ecc_pub_key = read_to_string("keys/lcalpubkey.txt").unwrap();
    let _ = socket.write_all(ecc_pub_key.as_bytes()).await;

}

fn load_remote_ecc_key() -> PublicKey { // loading the public key of the other host
    
    let pubhex = read_to_string("keys/remotepubkey.txt").unwrap();
    let pubbytes = hex::decode(pubhex.trim()).unwrap();
    let pubkey = PublicKey::from_sec1_bytes(&pubbytes).unwrap();

    pubkey

}

fn load_local_ecc_keys() -> (PublicKey, SecretKey) { // loading the asymmetric keys

    let sechex = read_to_string("keys/lcalseckey.txt").unwrap();
    let pubhex = read_to_string("keys/lcalpubkey.txt").unwrap();

    let secbytes = hex::decode(sechex.trim()).unwrap();
    let pubbytes = hex::decode(pubhex.trim()).unwrap();

    let seckey = SecretKey::from_slice(&secbytes).unwrap();
    let pubkey = PublicKey::from_sec1_bytes(&pubbytes).unwrap();

    (pubkey, seckey)

}