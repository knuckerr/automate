
use std::io;

use std::io::prelude::*;

use automate::database::database::DatabaseQuerys;

use automate::server_lib::server_lib::Target;
use std::net::TcpStream;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use nix::sys::signal::Signal;
use std::fs;
use std::fs::File;





#[derive(Debug, Clone)]
struct Inputs {
    fields : Vec<String>,
}

fn report(target:&String){
    let path = format!("report/{}",target);
    let folder = fs::create_dir_all(path.to_owned());
    if folder.is_ok(){
        let jobs = DatabaseQuerys::job_output(target,&"all".to_string());
        for job in jobs {
            let file_path = format!("{}/{}.txt",path,job.name);
            let file = File::create(file_path);
            if file.is_ok(){
                let mut file = file.unwrap();
                file.write_all(job.output.as_bytes()).unwrap();
            }else{
                eprintln!("{}",file.unwrap_err());
            }
        }

    }else{
        eprintln!("{}",folder.unwrap_err());
    }
}

fn logo(){
    println!(r#"
   _____          __                         __           
  /  _  \  __ ___/  |_  ____   _____ _____ _/  |_  ____   
 /  /_\  \|  |  \   __\/  _ \ /     \\__  \\   __\/ __ \  
/    |    \  |  /|  | (  <_> )  Y Y  \/ __ \|  | \  ___/  
\____|__  /____/ |__|  \____/|__|_|  (____  /__|  \___  > 
        \/                         \/     \/          \/  
             "#);
}

fn help(){
    print!("-------------------------------------------------------------------------\n");
    println!("{0: <10} {1:<35} {2:<10}","|Comands|","|Arguments|","|Description|");
    println!("-------------------------------------------------------------------------");
    print!("{0:<10}  {1:<35} {2:<10}","targets","","List of the Targets in Database\n");
    print!("{0:<10}  {1:<35} {2:<10}","services","$target_name","List of services\n");
    print!("{0:<10}  {1:<35} {2:<10}","jobs","$target_name","List of jobs\n");
    print!("{0:<10}  {1:<35} {2:<10}","kill","$job_pid","Stop a Jobs\n");
    print!("{0:<10}  {1:<35} {2:<10}","kill_all" ,"$target_name","Stop all Jobs\n");
    print!("{0:<10}  {1:<35} {2:<10}","report" ,"$target_name","Create report of all finish comands\n");
    print!("{0:<10}  {1:<35} {2:<10}","clear_all","$target_name","delete a target\n");
    print!("{0:<10}  {1:<35} {2:<10}","output","$jobname $target_name","Shows Results of a Jobs\n");
    print!("{0:<10}  {1:<35} {2:<10}","scan","name $target_name ip $target_ip","Begin a new scan \n");
    println!();
}


impl Inputs{
    fn vaild_input(self){
        match self.fields.len(){
            1 => match self.fields[0].as_str() {
                "targets" => {
                    let targets = DatabaseQuerys::list_targets();
                    for target in targets {
                        println!("ID:{}\tName:{}\tIP:{}",target.id ,target.name,target.ip);
                    }
                }
                "help" => {
                    help();
                }
                _ => {}
            }
            2 => match self.fields[0].as_str(){
                "services" => {
                    let target = &self.fields[1];
                    let ports = DatabaseQuerys::list_services(target);
                    for port in ports {
                        println!("---------------------------------------------------------------------------------------------------");
                        println!("Service:{}\tPort:{}\tStage:{}\tProtocol:{}\tVersion:{}",port.service,port.port,port.stage,port.protocol,port.version);
                    }
                    println!("-------------------------------------------------------------------------------------------------------");
                }
                "jobs" => {
                    let target = &self.fields[1];
                    let jobs = DatabaseQuerys::list_jobs(target);
                    for job in jobs {
                        println!("---------------------------------------------------------------------------------------------------");
                        println!("Pid:{}\tService:{}\tState:{}\tName:{}",job.id,job.service,job.state,job.name);
                    }
                    println!("-------------------------------------------------------------------------------------------------------");

                }
                "delete" => {
                    let target = &self.fields[1];
                    DatabaseQuerys::clear_target(target);
                    println!("Target {} have been deleted",target);
                }
                "report" => {
                    let target = &self.fields[1];
                    report(target);
                }
                "kill" => {
                    let job_pid : &i32  = &self.fields[1].parse().unwrap_or(0);
                    if *job_pid == 0 {
                        println!("Need to be A Number");
                    }
                    else {
                        let pid = Pid::from_raw(*job_pid);
                        let task_done = kill(pid,Signal::SIGKILL);
                        if task_done.is_ok(){
                            println!("Job {} Killed ",job_pid);
                        }
                        else{
                            println!("Job {} IS NOT RUNNING ",job_pid);
                        }

                    }

                }
                "kill_all" => {
                    let target = &self.fields[1];
                    let jobs = DatabaseQuerys::list_jobs(target);
                    for job in jobs {
                        let pid = Pid::from_raw(job.id);
                        let task_done = kill(pid,Signal::SIGKILL);
                        if task_done.is_ok(){
                            println!("Job {} Killed ",job.name);
                        }
                        else{
                            println!("Job {} IS NOT RUNNING ",job.name);
                        }


                    }
                }
                _ => {}
            }

            3 => match self.fields[0].as_str(){
                "output" => {
                    let job_name = &self.fields[1];
                    let target = &self.fields[2];
                    let jobs = DatabaseQuerys::job_output(target,job_name);
                    for job in jobs {
                        println!("---------------------------------------------------------------------------------------------------");
                        println!("Pid:{}\tService:{}\tState:{}\tName:{}\n{}",job.id,job.service,job.state,job.name,job.output);

                    }
                    println!("-------------------------------------------------------------------------------------------------------");

                }
                _ => {}
            }
            5 => {
                if self.fields[1] == "name" && self.fields[3] == "ip" && self.fields[0] == "scan" {
                    let name = self.fields[2].clone();
                    let ip = self.fields[4].clone();
                    let target = Target{name:name,ip:ip,id:0,command:"enumaration".to_string()};
                    let json = serde_json::to_string(&target).unwrap();
                    let stream = TcpStream::connect("127.0.0.1:1234");
                    if stream.is_ok(){
                        let send = format!("{}\n",json);
                        let mut stream = stream.unwrap();
                        let _ = stream.write(send.as_bytes());
                        let read = stream.read(&mut [0; 1024]).expect("cannot read from the server");
                        println!("{}",read.to_string());
                    }
                    else{
                        println!("Cannot connect to the Server ");
                    }
                }
            }
            _ => {}
        }

    }
}

fn main(){
    logo();
    help();
    loop {
        let mut input = String::new();
        print!("Automate > ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let data :Vec<_> = input.split_whitespace()
            .map(|x| x.to_string())
            .collect();
        let user_input = Inputs{fields:data};
        user_input.vaild_input();
    }
}