use std::net::UdpSocket;
use serde::{Serialize, Deserialize};
use std::net::{TcpStream, TcpListener};
use local_ip_address::local_ip;
use default_net::gateway::get_default_gateway;
use chrono::prelude::*;
use std::io::{Error, Write, Read};
use std::collections::HashMap;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use std::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Block {
    name: String,
    register: u16,
    count: u16,
}

impl Block {
    fn new(name: String, register: u16, count: u16) -> Block {
        Self {
            name,
            register,
            count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Register {
    name: String,
    scale: f32,
    unit: String,
    signed: bool,
}

impl Register {
    fn new(name: String, scale: f32, unit: String, signed: bool) -> Register {
        Self {
            name,
            scale,
            unit,
            signed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Invertor {
    pub registers: HashMap<u16, Register>,
    pub blocks: Vec<Block>,
}

impl Invertor {
    fn new() -> Invertor {
        let mut registers = HashMap::new();

        registers.insert(0x000B, Register::new(
            "product type".to_string(),
            1.0,
            "".to_string(),
            false,
        ));

        registers.insert(0x0100, Register::new(
            "battery SOC".to_string(),
            1.0,
            "%".to_string(),
            false,
        ));

        registers.insert(0x0101, Register::new(
            "battery voltage".to_string(),
            0.1,
            "V".to_string(),
            false,
        ));

        registers.insert(0x102, Register::new(
            "battery current".to_string(),
            0.1,
            "A".to_string(),
            false,
        ));

        let mut blocks = Vec::new();

        blocks.push(Block::new(
            "product_type".to_string(),
            0x000B,
            1,
        ));

        blocks.push(Block::new(
            "battery / pv".to_string(),
            0x0100,
            3,
        ));


        Self {
            registers,
            blocks
        }

    }
}

#[derive(Debug, Clone)]
pub struct TransactionIdGenerator {
    id: u16,
}

impl TransactionIdGenerator {
    fn new() -> Self {
        Self {
            id: 0,
        }
    }
    fn get_id(&mut self) -> u16 {
        self.id = self.id.wrapping_add(1);

        // I don't think the invertor liked a 0 id;
        if self.id == 0 {
            self.id = self.id.wrapping_add(1);
        }

        return self.id;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Frame {
    id: u16,
    code: u16,
    length: u16,
    addr: u8,
    f_code: u8,
    payload: Vec<u8>,
}

impl Frame {
    fn new(tid: u16, dev_code: u16, dev_addr: u8, f_code: u8, payload: Vec<u8>) -> Frame {
        
        Self {
            id: tid,
            code: dev_code,
            length: 2 + (payload.len() as u16),
            addr: dev_addr,
            f_code: f_code,
            payload: payload,
        }
    }

    fn header(&self) -> Vec<u8> {

        let mut header = Vec::new();

        header.append(&mut Vec::from(self.id.to_be_bytes()));
        header.append(&mut Vec::from(self.code.to_be_bytes()));
        header.append(&mut Vec::from(self.length.to_be_bytes()));
        header.append(&mut Vec::from(self.addr.to_be_bytes()));
        header.append(&mut Vec::from(self.f_code.to_be_bytes()));

        return header;
    }

    fn packet(&mut self) -> Vec<u8> {
        let mut packet = self.header();
        packet.append(&mut self.payload);

        return packet;
    }

    // No idea if this is right but we do at least return a thing
    fn crc16_modbus(data: Vec<u8>) -> Vec<u8> {

        let mut crc = 0xFFFF;

        for byte in data {

            crc ^= (byte as u16);

            for _ in 0..8 {

                crc = if (crc & 1) == 1 {
                    (crc >> 1) ^ 0xA001
                } else {
                    crc >> 1
                };
                
            }

        }

        return Vec::from(crc.to_be_bytes());
    }

    /*fn read_req(&mut self, dev_addr: u16, start: u16, count: u16) {
    }*/
}



fn heartbeat(tid: u16) -> Frame {

    let now = Utc::now();

    let mut payload = Vec::new();

    let year = (now.year() - 2000) as u8;

    let be_year: [u8; 1] = year.to_be_bytes();

    payload.append(&mut Vec::from(be_year));


    payload.append(&mut Vec::from((now.month() as u8).to_be_bytes()));
    payload.append(&mut Vec::from((now.day() as u8).to_be_bytes()));
    payload.append(&mut Vec::from((now.hour() as u8).to_be_bytes()));
    payload.append(&mut Vec::from((now.minute() as u8).to_be_bytes()));
    payload.append(&mut Vec::from((now.second() as u8).to_be_bytes()));

    // There's something I'm not getting about the way this works.
    payload.append(&mut Vec::from(60_u16.to_be_bytes()));

    return Frame::new(
        tid,
        0x0001,
        0xFF,
        0x01,
        payload,
    );
}

fn transact(mut socket: &TcpStream, mut command: Frame) -> Result<usize, Error> {

    let response = socket.write(command.packet().as_slice());
    return response;

    //return String::from("Hello");
}

fn main() -> std::io::Result<()> {

    let invertor = Invertor::new();


    println!("Invertor: {:?}", invertor);
    println!("Blocks: {:?}", invertor.blocks);

    let tcp_port = 8899;
    let udp_port = 58899;
    let default_gateway = get_default_gateway();

    let payload = Vec::from((0xFA01_u16).to_be_bytes());
    let crc = Frame::crc16_modbus(payload);
    println!("{:?}", crc);

    let Ok(default_gateway) = default_gateway else {
        return Ok(());
    };

    {
        //let mut heartbeat = heartbeat(1);

        let socket = UdpSocket::bind("0.0.0.0:34254")?;
        let my_ip_addr = local_ip();

        //let header = heartbeat.packet();

        let Ok(my_ip_addr) = my_ip_addr else {
            return Ok(());
        };

        let tcp_addr = format!("{}:{}", my_ip_addr, tcp_port);

        println!("TCP Address: {}", tcp_addr);

        let listener = TcpListener::bind(&tcp_addr);
        let Ok(listener) = listener else {
            println!("could not start tcp listener");
            return Ok(());
        };

        let cmd = format!("set>server={};", &tcp_addr);

        let cmd = cmd.into_bytes();

        let gateway_addr = format!("{}:{}", default_gateway.ip_addr, udp_port);

        println!("Initial Connection -> {}", gateway_addr);

        socket.send_to(&cmd, &gateway_addr).expect("Send to should succeed");

        println!("Pending...");

        let mut buf = [0; 10];

        socket.recv_from(&mut buf)?;

        let response = String::from_utf8((&buf).to_vec());

        let Ok(response) = response else {
            println!("Not a string.");
            return Ok(());
        };

        println!("Response: {}", response);

        let Ok((socket, _addr)) = listener.accept() else {
            println!("Could not establish connection.");
            return Ok(());
        };

        //let mut t_id = 1;
        //let rsp = transact(&socket, heartbeat(t_id));
        
        // what I want:
        // an u32
        // that I can grab from multiple threads
        // and when it's read it increases by one.
        // how do that?
        //let t_id = Arc::new(Mutex::new(1_u16));
        let t_id = Arc::new(Mutex::new(TransactionIdGenerator::new()));

        let (send_tx, send_rx) = mpsc::channel();

        // Send Heartbeat to Receiver;
        {
            let t_id_arc = Arc::clone(&t_id);

            let send_tx: std::sync::mpsc::Sender<Frame> = send_tx.clone();
            // I'm assuming this has to be lower than the heartbeat wait time because
            // heartbeat wait is expect a heartbeat in x, if not, stop.

            let heartbeat_delay = 30;

            // This is bob, bob is amazing.
            thread::spawn(move || {

                let mut seconds = 0;

                // Infinite loop gooooo!
                loop {

                    if seconds == 0 {

                        let t_id = {
                            let mut lock = t_id_arc.lock().unwrap();
                            (*lock).get_id()
                        };

                        println!("Sending Frame");
                        send_tx.send(heartbeat(t_id));
                        // t_id += 1;
                    }

                    thread::sleep(Duration::from_secs(1));
                    seconds += 1;
                    
                    if seconds >= heartbeat_delay {
                        seconds = 0;
                    }

                    
                }

            });
        }

        
        /* {
            let t_id_arc = Arc::clone(&t_id);
            let send_tx: std::sync::mpsc::Sender<Frame> = send_tx.clone();
            let info_delay = 10;

            //this is fred, he is a thread;
            thread::spawn(move || {

                let mut seconds = 0;

                loop {

                    if seconds == 0 {
                        let mut t_id = 1;
                        
                        {
                            let mut lock = t_id_arc.lock().unwrap();
                            t_id = (*lock).get_id();
                        }

                        println!("Sending Info Frame");
                        // (tid: u16, dev_code: u16, dev_addr: u8, f_code: u8, payload: Vec<u8>) -> Frame {

                        send_tx.send(Frame::new(
                            t_id,
                            0x0001,
                            0x0001, // Dev Addr
                            0x04,   // FCode
                            read_req(0x0001, 0x0001, 4);
                        ));
                    }

                    thread::sleep(Duration::from_secs(1));
                    seconds += 1;
                    
                    if seconds >= heartbeat_delay {
                        seconds = 0;
                    }
                }

            });
        } */


        // Send Block Request to invertor;
        {
            if let Ok(socket) = socket.try_clone() {
                thread::spawn(move || {
                    for received in send_rx {
                        println!("Frame Received: {:?}", received);
                        transact(&socket, received);
                    }
                });
            };
        }

        
        let (tx, rx) = mpsc::channel();

        {
            let tx: std::sync::mpsc::Sender<Frame> = tx.clone();
            if let Ok(mut socket) = socket.try_clone() {

                let mut resp_frame = Frame::new(
                    0,
                    0,
                    0,
                    0,
                    Vec::new()
                );

                thread::spawn(move || {
                    let mut buffer: [u8; 6] = [0; 6];
                    //let mut waiting = true;
                    let mut received = 0;
                    
                    while /*waiting && */ let Ok(len) = socket.read(&mut buffer) {

                        if len == 0 {
                            continue;
                        }

                        if resp_frame.id == 0 {

                            let t_id = ((buffer[0] as u16) << 8) | buffer[1] as u16;
                            let dev_code = ((buffer[2] as u16) << 8) | buffer[3] as u16;
                            let length = ((buffer[4] as u16) << 8) | buffer[5] as u16;

                            resp_frame.id = t_id;
                            resp_frame.code = dev_code;
                            resp_frame.length = length;

                        } else {
                            received = received + len;

                            // let's just assume address and fcode will never both be 0;
                            if resp_frame.addr == 0 && resp_frame.f_code == 0 {
                                resp_frame.addr = buffer[0];
                                resp_frame.f_code = buffer[1];

                                resp_frame.payload.extend_from_slice(&buffer[2..]);
                            } else {
                                resp_frame.payload.extend_from_slice(&buffer);
                            }

                            if received >= resp_frame.length.into() {
                                tx.send(resp_frame.clone());

                                received = 0;
                                resp_frame = Frame::new(
                                    0,
                                    0,
                                    0,
                                    0,
                                    Vec::new()
                                );

                            }
                        }

                        buffer = [0; 6];
                        thread::sleep(Duration::from_secs(1));

                    }
                    
                });
                
            }

        }

        for received in rx {
            println!("Frame from Invertor: {:?}", received);
        }

    }

    Ok(())

}
