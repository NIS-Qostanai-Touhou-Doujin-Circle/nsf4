use actix_web::{web, App, HttpServer, Responder, HttpResponse, post, get};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
// Если вы планируете использовать UUID для ID пользователей или комнат:
// use uuid::Uuid;

// Структура для общего состояния приложения
struct AppState {
    rooms: Mutex<HashMap<String, Room>>, // Ключ: Room ID, Значение: Room
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
    id: String, // Может быть UUID или имя пользователя
    // Можно добавить другие поля, например, display_name
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Participant {
    user: User,
    camera_on: bool,
    mic_on: bool,
    // Сюда позже можно будет добавить состояние WebRTC (например, PeerConnection, WebSocket actor)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Room {
    id: String,
    creator_id: String,
    participants: HashMap<String, Participant>, // Ключ: User ID, Значение: Participant
    pending_requests: HashMap<String, User>,  // Ключ: User ID, Значение: User, ожидающий одобрения
}

// --- Структуры для запросов и ответов API ---

#[derive(Deserialize)]
struct CreateRoomRequest {
    room_id: String,
    creator_id: String,
}

#[derive(Serialize)]
struct RoomResponse {
    id: String,
    creator_id: String,
    participants: Vec<Participant>,
    pending_requests: Vec<User>,
}

#[derive(Serialize)]
struct GeneralMessageResponse {
    message: String,
    room_id: Option<String>,
    user_id: Option<String>,
}


#[derive(Deserialize)]
struct JoinRoomRequest {
    user_id: String,
}

#[derive(Deserialize)]
struct ApproveOrDenyRequest {
    user_id_to_act_on: String,
    // В реальном приложении здесь должен быть ID того, кто одобряет/отклоняет,
    // чтобы проверить, что это создатель комнаты.
    // approver_id: String,
}


#[derive(Deserialize)]
struct MediaStateUpdateRequest {
    user_id: String,
    camera_on: Option<bool>,
    mic_on: Option<bool>,
}

#[derive(Serialize)]
struct MediaStateUpdateResponse {
    message: String,
    user_id: String,
    camera_on: bool,
    mic_on: bool,
}

#[derive(Deserialize)]
struct LeaveRoomRequest {
    user_id: String,
}

// --- Обработчики HTTP ---

// 1. Создание комнаты
#[post("/rooms")]
async fn create_room_handler(
    state: web::Data<AppState>,
    req_body: web::Json<CreateRoomRequest>,
) -> impl Responder {
    let mut rooms_guard = state.rooms.lock().unwrap();
    let room_id = req_body.room_id.clone();
    let creator_id = req_body.creator_id.clone();

    if rooms_guard.contains_key(&room_id) {
        return HttpResponse::Conflict().json(GeneralMessageResponse {
            message: "Room with this ID already exists".to_string(),
            room_id: Some(room_id),
            user_id: None,
        });
    }

    let creator_user = User { id: creator_id.clone() };
    let creator_participant = Participant {
        user: creator_user.clone(),
        camera_on: false, // Начальное состояние
        mic_on: false,    // Начальное состояние
    };

    let mut participants = HashMap::new();
    participants.insert(creator_id.clone(), creator_participant.clone());

    let new_room = Room {
        id: room_id.clone(),
        creator_id: creator_id.clone(),
        participants,
        pending_requests: HashMap::new(),
    };
    rooms_guard.insert(room_id.clone(), new_room.clone());

    HttpResponse::Created().json(RoomResponse {
        id: new_room.id,
        creator_id: new_room.creator_id,
        participants: new_room.participants.values().cloned().collect(),
        pending_requests: new_room.pending_requests.values().cloned().collect(),
    })
}

// 2. Запрос на присоединение к комнате
#[post("/rooms/{room_id}/join")]
async fn request_join_room_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<JoinRoomRequest>,
) -> impl Responder {
    let room_id = path.into_inner();
    let mut rooms_guard = state.rooms.lock().unwrap();
    let user_id_to_join = req_body.user_id.clone();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if room.participants.contains_key(&user_id_to_join) {
                return HttpResponse::Ok().json(GeneralMessageResponse {
                    message: "User already in room".to_string(),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_join),
                });
            }
            if room.pending_requests.contains_key(&user_id_to_join) {
                 return HttpResponse::Ok().json(GeneralMessageResponse {
                    message: "Join request already pending".to_string(),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_join),
                });
            }
            // Предполагаем, что user_id уникален и валиден
            let requesting_user = User { id: user_id_to_join.clone() };
            room.pending_requests.insert(user_id_to_join.clone(), requesting_user);

            // TODO: Уведомить создателя комнаты (room.creator_id) через WebSocket о новом запросе.
            println!("User {} requested to join room {}", user_id_to_join, room_id);

            HttpResponse::Ok().json(GeneralMessageResponse {
                message: "Join request sent. Waiting for approval.".to_string(),
                room_id: Some(room_id),
                user_id: Some(user_id_to_join),
            })
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// 2b. Одобрение запроса на присоединение (создателем комнаты)
#[post("/rooms/{room_id}/approve")]
async fn approve_join_request_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<ApproveOrDenyRequest>,
    // TODO: Добавить проверку, что вызывающий API является создателем комнаты (например, через токен аутентификации)
) -> impl Responder {
    let room_id = path.into_inner();
    let mut rooms_guard = state.rooms.lock().unwrap();
    let user_id_to_approve = req_body.user_id_to_act_on.clone();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            // В реальном приложении: проверить, что ID пользователя, делающего этот запрос, совпадает с room.creator_id
            if let Some(user_to_add) = room.pending_requests.remove(&user_id_to_approve) {
                let new_participant = Participant {
                    user: user_to_add.clone(),
                    camera_on: false,
                    mic_on: false,
                };
                room.participants.insert(user_id_to_approve.clone(), new_participant);

                // TODO: Уведомить одобренного пользователя через WebSocket.
                // TODO: Уведомить остальных участников комнаты через WebSocket о новом пользователе.
                println!("User {} approved for room {}", user_id_to_approve, room_id);

                HttpResponse::Ok().json(GeneralMessageResponse {
                    message: format!("User {} approved and added to room.", user_id_to_approve),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_approve),
                })
            } else {
                HttpResponse::NotFound().json(GeneralMessageResponse {
                    message: format!("No pending request found for user {}.", user_id_to_approve),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_approve),
                })
            }
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// 2c. Отклонение запроса на присоединение (создателем комнаты)
#[post("/rooms/{room_id}/deny")]
async fn deny_join_request_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<ApproveOrDenyRequest>,
    // TODO: Добавить проверку, что вызывающий API является создателем комнаты
) -> impl Responder {
    let room_id = path.into_inner();
    let mut rooms_guard = state.rooms.lock().unwrap();
    let user_id_to_deny = req_body.user_id_to_act_on.clone();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if room.pending_requests.remove(&user_id_to_deny).is_some() {
                // TODO: Уведомить отклоненного пользователя через WebSocket.
                 println!("User {} denied for room {}", user_id_to_deny, room_id);
                HttpResponse::Ok().json(GeneralMessageResponse {
                    message: format!("Join request for user {} denied.", user_id_to_deny),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_deny),
                })
            } else {
                HttpResponse::NotFound().json(GeneralMessageResponse {
                    message: format!("No pending request found for user {}.", user_id_to_deny),
                    room_id: Some(room_id),
                    user_id: Some(user_id_to_deny),
                })
            }
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}


// 3. Включение/выключение камеры/микрофона
#[post("/rooms/{room_id}/media_status")]
async fn update_media_status_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<MediaStateUpdateRequest>,
) -> impl Responder {
    let room_id = path.into_inner();
    let mut rooms_guard = state.rooms.lock().unwrap();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if let Some(participant) = room.participants.get_mut(&req_body.user_id) {
                if let Some(cam_status) = req_body.camera_on {
                    participant.camera_on = cam_status;
                }
                if let Some(mic_status) = req_body.mic_on {
                    participant.mic_on = mic_status;
                }

                // TODO: Разослать это изменение остальным участникам комнаты через WebSocket.
                println!("User {} in room {} updated media status: cam={}, mic={}",
                    req_body.user_id, room_id, participant.camera_on, participant.mic_on);

                HttpResponse::Ok().json(MediaStateUpdateResponse {
                    message: "Media status updated".to_string(),
                    user_id: req_body.user_id.clone(),
                    camera_on: participant.camera_on,
                    mic_on: participant.mic_on,
                })
            } else {
                HttpResponse::NotFound().json(GeneralMessageResponse {
                    message: "Participant not found in room".to_string(),
                    room_id: Some(room_id),
                    user_id: Some(req_body.user_id.clone()),
                })
            }
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// 6. Покинуть конференцию
#[post("/rooms/{room_id}/leave")]
async fn leave_room_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<LeaveRoomRequest>,
) -> impl Responder {
    let room_id = path.into_inner();
    let mut rooms_guard = state.rooms.lock().unwrap();

    match rooms_guard.get_mut(&room_id) {
        Some(room) => {
            if room.participants.remove(&req_body.user_id).is_some() {
                // TODO: Уведомить остальных участников через WebSocket, что этот пользователь покинул комнату.
                // TODO: Очистить ресурсы WebRTC для этого пользователя.
                println!("User {} left room {}", req_body.user_id, room_id);

                let room_is_now_empty = room.participants.is_empty();
                let mut message = format!("User {} left room.", req_body.user_id);

                if room_is_now_empty {
                    // Опционально: удалить комнату, если она пуста.
                    // rooms_guard.remove(&room_id);
                    // message = format!("User {} left room. Room is now empty and has been removed.", req_body.user_id);
                    message = format!("User {} left room. Room is now empty.", req_body.user_id);
                     println!("Room {} is now empty.", room_id);
                }

                HttpResponse::Ok().json(GeneralMessageResponse {
                    message,
                    room_id: Some(room_id),
                    user_id: Some(req_body.user_id.clone()),
                })
            } else {
                HttpResponse::BadRequest().json(GeneralMessageResponse {
                    message: format!("User {} not found in room.", req_body.user_id),
                    room_id: Some(room_id),
                    user_id: Some(req_body.user_id.clone()),
                })
            }
        }
        None => HttpResponse::NotFound().json(GeneralMessageResponse {
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}

// Получение информации о комнате (для отладки или клиентских нужд)
#[get("/rooms/{room_id}")]
async fn get_room_info_handler(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let room_id = path.into_inner();
    let rooms_guard = state.rooms.lock().unwrap();

    match rooms_guard.get(&room_id) {
        Some(room) => HttpResponse::Ok().json(RoomResponse {
            id: room.id.clone(),
            creator_id: room.creator_id.clone(),
            participants: room.participants.values().cloned().collect(),
            pending_requests: room.pending_requests.values().cloned().collect(),
        }),
        None => HttpResponse::NotFound().json(GeneralMessageResponse{
            message: "Room not found".to_string(),
            room_id: Some(room_id),
            user_id: None,
        }),
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Инициализация состояния приложения
    let app_state = web::Data::new(AppState {
        rooms: Mutex::new(HashMap::new()),
    });

    println!("Сервер запускается на http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone()) // Передача состояния в приложение
            .service(create_room_handler)
            .service(get_room_info_handler) // Добавлен для удобства просмотра состояния комнаты
            .service(
                web::scope("/rooms/{room_id}") // Группировка роутов для конкретной комнаты
                    .service(request_join_room_handler)
                    .service(approve_join_request_handler)
                    .service(deny_join_request_handler)
                    .service(update_media_status_handler)
                    .service(leave_room_handler)
            )
            // Сюда будет добавлен обработчик WebSocket:
            // .route("/ws/{room_id}/{user_id}", web::get().to(websocket_route_function))
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await
}