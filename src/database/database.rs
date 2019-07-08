extern crate postgres;
extern crate serde_json;

use crate::server_lib::server_lib::{Target,CommandRun,Ports};
use std::process::exit;
use self::postgres::{Connection, TlsMode};
use std::io;
use std::io::Read;
use std::fs::File;

#[derive(Debug, Clone, Deserialize)]
pub struct Postgres {
    host: String,
    port: i32,
    user: String,
    pass: String,
    database: String,
}



pub fn read_file(path: &str) -> Result<String, io::Error> {
    let mut data = String::new();
    File::open(path)?.read_to_string(&mut data)?;
    Ok(data)
}


pub struct DatabaseQuerys;


impl DatabaseQuerys {
    pub fn postgres() -> postgres::Connection {
        let json_file = read_file("src/database.json");
        if json_file.is_err() {
            println!("{}", json_file.unwrap_err());
            exit(1);
        }
        let postgres: Result<Postgres, serde_json::Error> =
            serde_json::from_str(&json_file.unwrap());
        if postgres.is_ok() {
            let postgres = postgres.unwrap();
            let data = format!(
                "postgres://{}:{}@{}:{}/{}",
                postgres.user,
                postgres.pass,
                postgres.host,
                postgres.port,
                postgres.database
            );
            let data_clone = data.clone();
            let conn = Connection::connect(data, TlsMode::None);
            if conn.is_ok() {
                conn.unwrap()
            } else {
                let err  =  format!("{}",conn.unwrap_err());
                let database_not_exist = format!(r#"database error: FATAL: database "{}" does not exist"#,postgres.database);
                if err == database_not_exist{
                    DatabaseQuerys::init_db(postgres);
                    let conn = Connection::connect(data_clone, TlsMode::None);
                    if conn.is_ok(){
                        conn.unwrap()
                    }
                    else{
                        println!("{}", err);
                        exit(1);
                    }
                }
                else {
                    println!("{}", err);
                    exit(1);
                }
            }
        } else {
            println!("{}", postgres.unwrap_err());
            exit(1);
        }
    }
    pub fn insert_target(target: &mut Target) {
        let conn = DatabaseQuerys::postgres();
        let data = conn.execute(
            "INSERT INTO target(name,ip) VALUES($1,$2)",
            &[&target.name, &target.ip],
        );
        if data.is_err() {
            eprintln!("{}", data.unwrap_err());
        } else {
            data.unwrap();
            for row in &conn.query(
                "SELECT id  FROM target WHERE name=$1 AND ip=$2",
                &[&target.name, &target.ip],
            ).unwrap()
            {
                target.id = row.get(0);
            }
        }
    }
    pub fn insert_services(ports: &[Ports], target: &Target) {
        let conn = DatabaseQuerys::postgres();
        for port in ports {
            let num_port: i32 = port.port.parse().unwrap();
            let data = conn.execute(
                "INSERT INTO ports
                (service,port,version,protocol,stage,target_name) VALUES
                ($1,$2,$3,$4,$5,$6)",
                &[
                    &port.service,
                    &num_port,
                    &port.version,
                    &port.protocol,
                    &port.stage,
                    &target.name,
                ],
            );
            if data.is_ok() {
                data.unwrap();
            } else {
                eprintln!("{}", data.unwrap_err());
            }

        }

    }


    pub fn list_services(target_name: &String) -> Vec<Ports> {
        let mut ports = Vec::new();
        let conn = DatabaseQuerys::postgres();
        let data = conn.query(
            "SELECT service,port,version,protocol,stage FROM ports WHERE target_name = $1",
            &[&target_name],
        );
        if data.is_ok() {
            for row in &data.unwrap() {
                let service: String = row.get(0);
                let port: i32 = row.get(1);
                let version: String = row.get(2);
                let protocol: String = row.get(3);
                let stage: String = row.get(4);
                let port = Ports {
                    service: service,
                    port: port.to_string(),
                    version: version,
                    protocol: protocol,
                    stage: stage,
                };
                ports.push(port);
            }
            ports
        } else {
            eprintln!("{}", data.unwrap_err());
            ports
        }

    }

    pub fn list_targets() -> Vec<Target> {
        let mut targets = Vec::new();
        let conn = DatabaseQuerys::postgres();
        let data = conn.query("SELECT name,ip,id FROM target", &[]);
        if data.is_ok() {
            for row in &data.unwrap() {
                let name: String = row.get(0);
                let ip: String = row.get(1);
                let id: i32 = row.get(2);
                let target = Target {
                    ip: ip,
                    name: name,
                    id: id,
                    command: "".to_string(),
                };
                targets.push(target);
            }
            targets
        } else {
            eprintln!("{}", data.unwrap_err());
            targets
        }

    }

    pub fn list_jobs(target_name: &String) -> Vec<CommandRun> {
        let mut commands = Vec::new();
        let conn = DatabaseQuerys::postgres();
        let data = conn.query(
            "SELECT job_pid,state,service,name FROM jobs WHERE target_name = $1
                              AND job_pid IS NOT NULL",
            &[&target_name],
        );
        if data.is_ok() {
            for row in &data.unwrap() {
                let id: i32 = row.get(0);
                let state: String = row.get(1);
                let service: String = row.get(2);
                let name: String = row.get(3);
                let command = CommandRun {
                    id: id,
                    state: state,
                    service: service,
                    name: name,
                    ..Default::default()
                };
                commands.push(command);
            }
            commands
        } else {
            eprintln!("{}", data.unwrap_err());
            commands
        }

    }


    pub fn job_output(target_name: &String,job_name: &String) -> Vec<CommandRun> {
        let mut commands = Vec::new();
        let conn = DatabaseQuerys::postgres();
        let mut query = String::new();
        if job_name == "all"{
            query.push_str("SELECT job_pid,state,service,name,output FROM jobs WHERE target_name = $1
            AND state = 'finish'
            AND output IS NOT NULL
            AND job_pid IS NOT NULL");

        }else{
            query.push_str("SELECT job_pid,state,service,name,output FROM jobs WHERE target_name = $1
            AND name = $2 AND state = 'finish'
            AND output IS NOT NULL
            AND job_pid IS NOT NULL");

        }
        let data = if job_name == "all"{
            conn.query(&query,
            &[&target_name])
        }else{
            conn.query(&query,
            &[&target_name,&job_name])
        };
        if data.is_ok() {
            for row in &data.unwrap() {
                let id: i32 = row.get(0);
                let state: String = row.get(1);
                let service: String = row.get(2);
                let name: String = row.get(3);
                let output: String = row.get(4);
                let command = CommandRun {
                    id: id,
                    state: state,
                    service: service,
                    name: name,
                    output:output,
                    ..Default::default()
                };
                commands.push(command);
            }
            commands
        } else {
            eprintln!("{}", data.unwrap_err());
            commands
        }

    }

    pub fn clear_target(target:&String){
        let conn = DatabaseQuerys::postgres();
        let queries = ["DELETE FROM target WHERE name = $1",
        "DELETE FROM exploits WHERE target_name = $1",
        "DELETE FROM ports WHERE target_name = $1",
        "DELETE FROM urls WHERE target_name = $1",
        "DELETE FROM dns WHERE target_name = $1",
        "DELETE FROM jobs WHERE target_name = $1"];
        for query in &queries {
            let data = conn.query(query,&[&target]);
            if data.is_err(){
                eprintln!("{}",data.unwrap_err());
            }else{
                data.unwrap();
            }
        }
    }

    pub fn init_db(postgres : Postgres) {
        let data = format!(
            "postgres://{}:{}@{}:{}",
            postgres.user,
            postgres.pass,
            postgres.host,
            postgres.port
            );
        let conn = Connection::connect(data, TlsMode::None).unwrap();
        let database_str = format!(r#"CREATE DATABASE "{}" "#,postgres.database);

        let database_drop_str = format!(r#"DROP DATABASE "{}" "#,postgres.database);
        let database = conn.query(&database_drop_str,&[]);
        if database.is_ok(){
            database.unwrap();
        }
        let database = conn.query(&database_str,&[]);
        if database.is_ok() {
            let data = format!(
                "postgres://{}:{}@{}:{}/{}",
                postgres.user,
                postgres.pass,
                postgres.host,
                postgres.port,
                postgres.database
                );
            let conn = Connection::connect(data, TlsMode::None).unwrap();
            let queries = ["CREATE TABLE target (
            id serial PRIMARY KEY NOT NULL,
            name text NULL,
             ip varchar NULL)",

            "CREATE TABLE exploits (
            id serial PRIMARY KEY NOT NULL,
             name text NULL,
             output varchar NULL,
             target_name varchar NULL)",


            "CREATE TABLE ports (
            id serial PRIMARY KEY NOT NULL,
            service varchar NULL,
            port int4 NULL,
            version varchar NULL,
            protocol varchar NULL,
            target_name varchar NULL,
            stage varchar NULL)",


            "CREATE TABLE urls (
            id serial PRIMARY KEY NOT NULL,
            url varchar,
            target_name varchar,
            response_code int4)",



            "CREATE TABLE dns (
            id serial PRIMARY KEY NOT NULL,
            url varchar,
            target_name varchar,
            ip varchar)",


            "CREATE TABLE exploitdb (
            id serial PRIMARY KEY NOT NULL,
            file varchar NOT NULL,
            description varchar NOT NULL,
            date varchar NOT NULL,
            author varchar NOT NULL,
            type varchar NOT NULL,
            platform varchar NOT NULL,
            port varchar NOT NULL,
            output varchar NOT NULL)",


            "CREATE TABLE jobs (
            id serial PRIMARY KEY NOT NULL,
            target_id int4 NULL,
            job_pid int4 NULL,
            state text NULL,
            service text NULL,
            output text NULL,
            error_msg varchar NULL,
            name varchar NULL,
            target_name varchar NULL);"];
            for query in queries.iter() {
                let data = conn.execute(query,&[]);
                if data.is_ok(){
                    data.unwrap();
                }
                else {
                    println!("{}",data.unwrap_err());
                }
            }

        }
        else {
            eprintln!("{}",database.unwrap_err());
        }

    }

    pub fn new_jobs(command: &mut CommandRun, target: &Target, status: &str) {
        match status {
            "start" => {
                let conn = DatabaseQuerys::postgres();
                let data = conn.execute(
                    "INSERT INTO
                    jobs(target_id,job_pid,state,service,name,target_name)
                    VALUES($1,$2,$3,$4,$5,$6)",
                    &[
                        &target.id,
                        &command.id,
                        &status,
                        &command.service,
                        &command.name,
                        &target.name,
                    ],
                );
                if data.is_err() {
                    eprintln!("{}", data.unwrap_err());
                } else {
                    data.unwrap();
                }
            }
            "error_start" => {
                let conn = DatabaseQuerys::postgres();
                let data = conn.execute(
                    "INSERT INTO
                    jobs(target_id,job_pid,state,
                    service,name,target_name,error_msg)
                    VALUES($1,$2,$3,$4,$5,$6,$7)",
                    &[
                        &target.id,
                        &command.id,
                        &status,
                        &command.service,
                        &command.name,
                        &target.name,
                        &command.output,
                    ],
                );
                if data.is_err() {
                    eprintln!("{}", data.unwrap_err());
                } else {
                    data.unwrap();
                }

            }
            "finish" => {
                let conn = DatabaseQuerys::postgres();
                let data = conn.execute(
                    "UPDATE jobs SET state=$1 ,output=$2 WHERE target_id = $3 AND job_pid = $4",
                    &[&status, &command.output, &target.id, &command.id],
                );
                if data.is_err() {
                    eprintln!("{}", data.unwrap_err());
                } else {
                    data.unwrap();
                }
            }

            "error_output" => {
                let conn = DatabaseQuerys::postgres();
                let data = conn.execute(
                    "UPDATE jobs SET state=$1 ,error_msg=$2 WHERE target_id=$3 AND job_pid=$4",
                    &[&status, &command.output, &target.id, &command.id],
                );
                if data.is_err() {
                    eprintln!("{}", data.unwrap_err());
                } else {
                    data.unwrap();
                }
            }
            _ => {}
        }
    }
}
