use high_performance_server::{
    Router, MiddlewareChain, Request, Response, Status,
    logging_middleware, cors_middleware, content_type_middleware,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A simple in-memory database for the API server example
struct Database {
    users: RwLock<HashMap<String, User>>,
}

/// A user in the database
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    name: String,
    email: String,
}

impl Database {
    fn new() -> Self {
        let mut users = HashMap::new();
        
        // Add some sample users
        users.insert(
            "1".to_string(),
            User {
                id: "1".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
        );
        
        users.insert(
            "2".to_string(),
            User {
                id: "2".to_string(),
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        );
        
        Self {
            users: RwLock::new(users),
        }
    }
    
    fn get_user(&self, id: &str) -> Option<User> {
        let users = self.users.read().unwrap();
        users.get(id).cloned()
    }
    
    fn get_all_users(&self) -> Vec<User> {
        let users = self.users.read().unwrap();
        users.values().cloned().collect()
    }
    
    fn create_user(&self, user: User) -> User {
        let mut users = self.users.write().unwrap();
        users.insert(user.id.clone(), user.clone());
        user
    }
    
    fn update_user(&self, id: &str, user: User) -> Option<User> {
        let mut users = self.users.write().unwrap();
        if users.contains_key(id) {
            users.insert(id.to_string(), user.clone());
            Some(user)
        } else {
            None
        }
    }
    
    fn delete_user(&self, id: &str) -> Option<User> {
        let mut users = self.users.write().unwrap();
        users.remove(id)
    }
}

fn main() {
    // Create a new router
    let mut router = Router::new();
    
    // Create a database
    let db = Arc::new(Database::new());
    
    // Set up routes
    
    // GET /users - Get all users
    let router_clone1 = router.clone();
    let db_clone = db.clone();
    router.get("/users", move |_req| {
        let users = db_clone.get_all_users();
        let json = serde_json::to_string(&users).unwrap();
        
        let mut response = Response::new(Status::Ok);
        response.set_header("Content-Type", "application/json");
        response.set_body(json.as_bytes());
        
        Ok(response)
    });
    
    // GET /users/:id - Get a specific user
    let router_clone2 = router.clone();
    let db_clone = db.clone();
    router.get("/users/:id", move |req| {
        // Extract the user ID from the URL
        let params = router_clone2.extract_params("/users/:id", &req.uri);
        let id = params.get("id").unwrap();
        
        // Look up the user
        match db_clone.get_user(id) {
            Some(user) => {
                let json = serde_json::to_string(&user).unwrap();
                
                let mut response = Response::new(Status::Ok);
                response.set_header("Content-Type", "application/json");
                response.set_body(json.as_bytes());
                
                Ok(response)
            }
            None => {
                let mut response = Response::new(Status::NotFound);
                response.set_body(format!("User with ID {} not found", id).as_bytes());
                
                Ok(response)
            }
        }
    });
    
    // POST /users - Create a new user
    let router_clone3 = router.clone();
    let db_clone = db.clone();
    router.post("/users", move |req| {
        // Parse the user from the request body
        match serde_json::from_slice::<User>(&req.body) {
            Ok(user) => {
                let created_user = db_clone.create_user(user);
                let json = serde_json::to_string(&created_user).unwrap();
                
                let mut response = Response::new(Status::Created);
                response.set_header("Content-Type", "application/json");
                response.set_body(json.as_bytes());
                
                Ok(response)
            }
            Err(e) => {
                let mut response = Response::new(Status::BadRequest);
                response.set_body(format!("Invalid user data: {}", e).as_bytes());
                
                Ok(response)
            }
        }
    });
    
    // PUT /users/:id - Update a user
    let router_clone4 = router.clone();
    let db_clone = db.clone();
    router.put("/users/:id", move |req| {
        // Extract the user ID from the URL
        let params = router_clone4.extract_params("/users/:id", &req.uri);
        let id = params.get("id").unwrap();
        
        // Parse the user from the request body
        match serde_json::from_slice::<User>(&req.body) {
            Ok(user) => {
                match db_clone.update_user(id, user) {
                    Some(updated_user) => {
                        let json = serde_json::to_string(&updated_user).unwrap();
                        
                        let mut response = Response::new(Status::Ok);
                        response.set_header("Content-Type", "application/json");
                        response.set_body(json.as_bytes());
                        
                        Ok(response)
                    }
                    None => {
                        let mut response = Response::new(Status::NotFound);
                        response.set_body(format!("User with ID {} not found", id).as_bytes());
                        
                        Ok(response)
                    }
                }
            }
            Err(e) => {
                let mut response = Response::new(Status::BadRequest);
                response.set_body(format!("Invalid user data: {}", e).as_bytes());
                
                Ok(response)
            }
        }
    });
    
    // DELETE /users/:id - Delete a user
    let router_clone5 = router.clone();
    let db_clone = db.clone();
    router.delete("/users/:id", move |req| {
        // Extract the user ID from the URL
        let params = router_clone5.extract_params("/users/:id", &req.uri);
        let id = params.get("id").unwrap();
        
        // Delete the user
        match db_clone.delete_user(id) {
            Some(_) => {
                let mut response = Response::new(Status::NoContent);
                Ok(response)
            }
            None => {
                let mut response = Response::new(Status::NotFound);
                response.set_body(format!("User with ID {} not found", id).as_bytes());
                
                Ok(response)
            }
        }
    });
    
    // Create a middleware chain
    let mut chain = MiddlewareChain::new();
    
    // Add middleware
    chain.add(logging_middleware);
    chain.add(cors_middleware(vec!["*".to_string()]));
    chain.add(content_type_middleware("application/json".to_string()));
    
    // Set the router as the final handler
    let router_arc = Arc::new(router);
    chain.set_handler(move |req| {
        router_arc.handle_request(req)
    });
    
    println!("API server example");
    println!("Available routes:");
    println!("  GET    /users");
    println!("  GET    /users/:id");
    println!("  POST   /users");
    println!("  PUT    /users/:id");
    println!("  DELETE /users/:id");
    println!();
    println!("To test the routes, you can use curl:");
    println!("  curl http://localhost:8080/users");
    println!("  curl http://localhost:8080/users/1");
    println!("  curl -X POST -H \"Content-Type: application/json\" -d '{{\"id\":\"3\",\"name\":\"Charlie\",\"email\":\"charlie@example.com\"}}' http://localhost:8080/users");
    println!("  curl -X PUT -H \"Content-Type: application/json\" -d '{{\"id\":\"1\",\"name\":\"Alice Updated\",\"email\":\"alice@example.com\"}}' http://localhost:8080/users/1");
    println!("  curl -X DELETE http://localhost:8080/users/2");
    
    // In a real server, this would be integrated with the event loop,
    // but for this example we'll just simulate a few requests
    
    // Simulate GET /users
    let request = Request::new(high_performance_server::Method::Get, "/users");
    match chain.handle(&request) {
        Ok(response) => {
            println!("\nSimulating GET /users:");
            println!("Status: {}", response.status as u16);
            println!("Body: {}", String::from_utf8_lossy(&response.body));
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    
    // Simulate GET /users/1
    let request = Request::new(high_performance_server::Method::Get, "/users/1");
    match chain.handle(&request) {
        Ok(response) => {
            println!("\nSimulating GET /users/1:");
            println!("Status: {}", response.status as u16);
            println!("Body: {}", String::from_utf8_lossy(&response.body));
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    
    // Simulate POST /users
    let mut request = Request::new(high_performance_server::Method::Post, "/users");
    let new_user = User {
        id: "3".to_string(),
        name: "Charlie".to_string(),
        email: "charlie@example.com".to_string(),
    };
    request.body = serde_json::to_vec(&new_user).unwrap();
    request.set_header("Content-Type", "application/json");
    
    match chain.handle(&request) {
        Ok(response) => {
            println!("\nSimulating POST /users:");
            println!("Status: {}", response.status as u16);
            println!("Body: {}", String::from_utf8_lossy(&response.body));
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    
    // Verify the new user was added with GET /users/3
    let request = Request::new(high_performance_server::Method::Get, "/users/3");
    match chain.handle(&request) {
        Ok(response) => {
            println!("\nSimulating GET /users/3:");
            println!("Status: {}", response.status as u16);
            println!("Body: {}", String::from_utf8_lossy(&response.body));
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}