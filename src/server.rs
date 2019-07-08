
extern crate serde_derive;
extern crate serde;
extern crate serde_json;



use automate::server_lib::server_lib::{Target, CommandRun,Enumaration};

use automate::database::database::DatabaseQuerys;

extern crate tokio;

///TOKIO
extern crate tokio_threadpool;

use self::tokio::prelude::*;
use self::tokio::io as tokio_io;
use self::tokio::net::TcpListener;
use self::tokio_threadpool::ThreadPool;
/////
///futures
extern crate futures;
use self::futures::{Future, lazy};
///
///
///postgress

///

use std::io::BufReader;
use std::process::{Command, Stdio};
use std::sync::Arc;



fn spawn_commands(command: &mut CommandRun, target: &Target) {
    let output = Command::new(command.command.to_owned())
        .args(command.args.to_owned())
        .stdout(Stdio::piped())
        .spawn();
    match output {
        Ok(output) => {
            command.id = output.id() as i32;
            DatabaseQuerys::new_jobs(command, target, "start");
            let child = output.wait_with_output();
            if child.is_err() {
                command.output = child.unwrap_err().to_string();
                DatabaseQuerys::new_jobs(command, target, "error_output");
            } else {
                command.output = String::from_utf8(child.unwrap().stdout).unwrap();
                DatabaseQuerys::new_jobs(command, target, "finish");
            }
        }

        Err(output) => {
            command.output = output.to_string();
            DatabaseQuerys::new_jobs(command, target, "error_start");

        }
    }
}

fn prepare_target(target: &Arc<Target>) {
    let mut nmap_run = CommandRun {
        output: "".to_string(),
        id: 0,
        name: "Nmap_scan".to_string(),
        args: vec![
            "-T4".to_string(),
            "-p-".to_string(),
            target.ip.to_owned(),
            "-A".to_string(),
            "-v".to_string(),
        ],
        command: "nmap".to_string(),
        logfile: format!("report/nmap/{}.txt", target.ip),
        service: "enumaration".to_string(),
        state : "none".to_string()
    };
    spawn_commands(&mut nmap_run, &target);
    let commands_to_run = Enumaration::start(&nmap_run.output, &target);
    if !commands_to_run.is_empty() {
        let pool = ThreadPool::new();
        for command in commands_to_run {
            let target_clone = target.clone();
            pool.spawn(lazy(move || {
                spawn_commands(&mut command.to_owned(), &target_clone);
                Ok(())
            }));
        }
        pool.shutdown_on_idle();
    } else {
        println!("Missing the comands.json file ")
    }
}

//read the buffer from the client and make it to struct if is vaild continue
//else send the error msg back to the client
fn read_client(item: &str) -> String {
    let target: Result<Target, serde_json::Error> = serde_json::from_str(&item);
    if target.is_ok() {
        let mut target = target.unwrap();
        let pool = ThreadPool::new();
        let screen = format!(
            r#"{{"status":"receive","target":{},"command":"enumaration" }}"#,
            target.ip
        );
        DatabaseQuerys::insert_target(&mut target);
        let target = Arc::new(target);
        pool.spawn(lazy(move || {
            prepare_target(&target);
            Ok(())
        }));
        pool.shutdown_on_idle();
        screen
    } else {
        eprintln!("{}", target.unwrap_err());
        "error".to_string()
    }
}

fn main() {
    let addr = "127.0.0.1:1234".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("unable to bind TCP listener");
    let server = listener
        .incoming()
        .map_err(|e| eprintln!("Accept Failed = {:?}", e))
        .for_each(|socket| {
            let (reader, writer) = socket.split();
            let lines = tokio_io::lines(BufReader::new(reader));
            let response = lines.map(|data| read_client(&data));
            let writes = response.fold(writer, |writer, response| {
                tokio_io::write_all(writer, response.into_bytes()).map(|(w, _)| w)
            });

            let msg = writes.then(move |_| Ok(()));
            tokio::spawn(msg)


        });

    tokio::run(server);

}
