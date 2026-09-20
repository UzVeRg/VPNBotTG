use std::{
    io::{Read, Write},
    net::TcpListener,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::{HwidDevice, RemnawaveClient};

pub struct Request {
    pub headers: String,
    pub body: String,
}

pub fn mock_api(responses: Vec<(u16, Value)>) -> (RemnawaveClient, JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, response) in responses {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "Ожидаемый запрос не поступил");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let header_end = loop {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let headers = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
            let length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            while bytes.len() < header_end + length {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            requests.push(Request {
                headers,
                body: String::from_utf8(bytes[header_end..header_end + length].to_vec()).unwrap(),
            });
            let body = response.to_string();
            write!(stream,
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len(),
            ).unwrap();
        }
        requests
    });
    let client = RemnawaveClient {
        http: reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
        base_url,
        token: String::from("test-token"),
    };
    (client, handle)
}

pub fn user(id: i64, telegram_id: u64) -> Value {
    json!({
        "id": id,
        "username": "test-user",
        "status": "ACTIVE",
        "trafficLimitBytes": 0,
        "expireAt": "2026-10-01T00:00:00Z",
        "telegramId": telegram_id,
        "hwidDeviceLimit": 5,
        "subscriptionUrl": "https://example.invalid/subscription",
        "tag": "PAID",
        "userTraffic": {
            "usedTrafficBytes": 0,
            "lifetimeUsedTrafficBytes": 0,
            "onlineAt": null,
            "firstConnectedAt": null,
        },
    })
}

pub fn device_json(user_id: i64, hwid: &str) -> Value {
    json!({
        "userId": user_id,
        "hwid": hwid,
        "platform": "Android",
        "osVersion": "16",
        "deviceModel": "Телефон",
        "userAgent": null,
        "createdAt": "2026-09-16T00:00:00Z",
        "updatedAt": "2026-09-16T01:00:00Z",
    })
}

pub fn device(user_id: i64, hwid: &str) -> HwidDevice {
    serde_json::from_value(device_json(user_id, hwid)).unwrap()
}

pub fn users_response(users: Vec<Value>) -> (u16, Value) {
    (200, json!({"response": {"users": users}}))
}

pub fn devices_response(devices: Vec<Value>) -> (u16, Value) {
    (
        200,
        json!({"response": {"total": devices.len(), "devices": devices}}),
    )
}
