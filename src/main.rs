extern crate actix_web;

use actix_rt::System;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::{future::ok, Future};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

struct User {
    name: String,
    ip: SocketAddr,
}

struct ChatMessage {
    name: String,
    mes: String,
}

struct ChatData {
    connected_users: HashMap<u32, User>,
    messages: Vec<ChatMessage>,
    id_cnt: u32,
    info_cnt: u32,
}

impl ChatData {
    fn create() -> ChatData {
        ChatData {
            connected_users: HashMap::new(),
            messages: Vec::new(),
            id_cnt: 0,
            info_cnt: 0,
        }
    }

    fn gen_user_id(&mut self) -> u32 {
        self.id_cnt = self.id_cnt + 1;
        self.id_cnt
    }

    fn get_user_id(&self, user: &String) -> Option<u32> {
        for (id, u) in self.connected_users.iter() {
            if &u.name == user {
                return Some(*id);
            }
        }

        None
    }

    fn is_user_connected(&self, name: &String) -> bool {
        for (_, user) in self.connected_users.iter() {
            if &user.name == name {
                return true;
            }
        }

        return false;
    }

    fn connect_user(&mut self, id: u32, user: User) {
        self.connected_users.insert(id, user);
    }

    fn add_message(&mut self, id: u32, mes: String) {
        let name = &self.connected_users.get(&id).unwrap().name;
        self.messages.push(ChatMessage {
            name: name.to_string(),
            mes,
        });
    }

    fn chat_html(&self) -> String {
        let mut html = String::new();

        for mes in self.messages.iter() {
            let text = format!("{}: {}<br/>", mes.name, mes.mes);
            html.push_str(&text);
        }

        html
    }
}

fn chat(
    _req: HttpRequest,
    data: web::Data<Arc<RwLock<ChatData>>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("Chat requested");

    let data = (*data).read().unwrap();
    let chat_html = data.chat_html();
    ok(HttpResponse::Ok().content_type("text/html").body(chat_html))
}

fn info(
    _req: HttpRequest,
    data: web::Data<Arc<RwLock<ChatData>>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("Info requested");
    let mut info = String::new();

    let mut data = (*data).write().unwrap();

    info.push_str("Connected users: <br/>");
    for (id, user) in data.connected_users.iter() {
        let text = format!("{} {} {}<br/>", id, user.name, user.ip);
        info.push_str(&text);
    }

    info.push_str(&format!("Id counter: {}<br/>", data.id_cnt));
    info.push_str(&format!("Info counter: {}<br/>", data.info_cnt));

    data.info_cnt = data.info_cnt + 1;

    ok(HttpResponse::Ok().content_type("text/html").body(info))
}

fn receive_message(
    req: HttpRequest,
    data: web::Data<Arc<RwLock<ChatData>>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let user = req.match_info().get("user").unwrap().to_string();
    let message = req.match_info().get("message").unwrap();

    let mut data = (*data).write().unwrap();
    if !data.is_user_connected(&user) {
        return ok(HttpResponse::BadRequest().body("User not connected"));
    }

    if let Some(id) = data.get_user_id(&user) {
        data.add_message(id, message.to_string());
    }

    ok(HttpResponse::Ok().body("Message received"))
}

fn connect_user(
    req: HttpRequest,
    data: web::Data<Arc<RwLock<ChatData>>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let user = String::from(req.match_info().get("user").unwrap());
    let ip = req.head().peer_addr.unwrap();

    let mut data = (*data).write().unwrap();

    if data.is_user_connected(&user) {
        return ok(HttpResponse::BadRequest().body("User already connected"));
    }

    let id = data.gen_user_id();
    println!("Connect user with id {}", id);
    let new_user = User { name: user, ip: ip };
    data.connect_user(id, new_user);

    ok(HttpResponse::Ok().body("User connected"))
}

fn main() -> std::io::Result<()> {
    let sys = System::new("chat-server");

    let data = Arc::new(RwLock::new(ChatData::create()));

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .service(web::resource("/chat.html").to_async(chat))
            .service(web::resource("/connect/{user}").to_async(connect_user))
            .service(web::resource("/chat/send/{user}/{message}").to_async(receive_message))
            .service(web::resource("/info.html").to_async(info))
    })
    .bind("127.0.0.25:8080")?
    .start();

    sys.run()
}
