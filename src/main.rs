extern crate actix_web;

use actix_rt::System;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::{future::ok, Future};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::SocketAddr;

struct User {
    name: String,
    ip: SocketAddr,
}

struct ChatMessage {
    name: String,
    mes: String,
}

struct ChatData {
    connected_users: RefCell<HashMap<u32, User>>,
    id_cnt: Cell<u32>,
    messages: RefCell<Vec<ChatMessage>>,
    info_cnt: Cell<u32>,
}

impl ChatData {
    fn create() -> ChatData {
        println!("Creating data");

        ChatData {
            connected_users: RefCell::new(HashMap::new()),
            id_cnt: Cell::new(0),
            messages: RefCell::new(Vec::new()),
            info_cnt: Cell::new(0),
        }
    }

    fn gen_user_id(&self) -> u32 {
        self.id_cnt.set(self.id_cnt.get() + 1);
        self.id_cnt.get()
    }

    fn get_user_id(&self, user: &String) -> Option<u32> {
        let users = self.connected_users.borrow();
        for (id, u) in users.iter() {
            if &u.name == user {
                return Some(*id);
            }
        }

        None
    }

    fn is_user_connected(&self, name: &String) -> bool {
        let users = self.connected_users.borrow();
        for (_, u) in users.iter() {
            if &u.name == name {
                return true;
            }
        }

        false
    }

    fn connect_user(&self, id: u32, user: User) {
        let mut users = self.connected_users.borrow_mut();
        users.insert(id, user);
    }

    fn add_message(&self, id: u32, mes: String) {
        let users = self.connected_users.borrow();
        let name = &users.get(&id).unwrap().name;

        let mut messages = self.messages.borrow_mut();
        messages.push(ChatMessage {
            name: name.to_string(),
            mes,
        });
    }

    fn chat_html(&self) -> String {
        let mut html = String::new();

        for mes in self.messages.borrow().iter() {
            let text = format!("{}: {}<br/>", mes.name, mes.mes);
            html.push_str(&text);
        }

        html
    }
}

fn chat(
    _req: HttpRequest,
    data: web::Data<ChatData>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    println!("Chat requested");

    let chat_html = data.chat_html();
    ok(HttpResponse::Ok().content_type("text/html").body(chat_html))
}

fn info(
    _req: HttpRequest,
    data: web::Data<ChatData>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let mut info = String::new();

    info.push_str("Connected users: <br/>");
    for (id, user) in data.connected_users.borrow().iter() {
        let text = format!("{} {} {}<br/>", id, user.name, user.ip);
        info.push_str(&text);
    }

    info.push_str(&format!("Id counter: {} <br/>", data.id_cnt.get()));
    info.push_str(&format!("Info counter: {} <br/>", data.info_cnt.get()));

    data.info_cnt.set(data.info_cnt.get() + 1);

    ok(HttpResponse::Ok().content_type("text/html").body(info))
}

fn receive_message(
    req: HttpRequest,
    data: web::Data<ChatData>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let user = req.match_info().get("user").unwrap().to_string();
    let message = req.match_info().get("message").unwrap();

    if !data.is_user_connected(&user) {
        return ok(HttpResponse::BadRequest().body("User not connected"));
    }

    if let Some(id) = data.get_user_id(&user) {
        data.add_message(id, message.to_string());
    }

    ok(HttpResponse::Ok().body("OK"))
}

fn connect_user(
    req: HttpRequest,
    data: web::Data<ChatData>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let user = String::from(req.match_info().get("user").unwrap());
    let ip = req.head().peer_addr.unwrap();

    if data.is_user_connected(&user) {
        return ok(HttpResponse::BadRequest().body("User already connected"));
    }

    let id = data.gen_user_id();
    println!("Connect user with id {}", id);
    let new_user = User { name: user, ip: ip };
    data.connect_user(id, new_user);

    ok(HttpResponse::Ok().body("ok"))
}

fn main() -> std::io::Result<()> {
    let sys = System::new("chat-server");

    HttpServer::new(|| {
        App::new()
            .data(ChatData::create())
            .service(web::resource("/chat.html").to_async(chat))
            .service(web::resource("/connect/{user}").to_async(connect_user))
            .service(web::resource("/chat/send/{user}/{message}").to_async(receive_message))
            .service(web::resource("/info.html").to_async(info))
    })
    .bind("127.0.0.25:8080")?
    .start();

    sys.run()
}
