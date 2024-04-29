mod pkg;
mod infra;
mod service;

fn main(){
    wd_log::log_info_ln!("start work...");
    service::start_pingora();
    wd_log::log_info_ln!("application over!!!");
}
