fn main() {
    let target = std::env::args().last().unwrap();
    if target.contains("hang") {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
    if target.contains("fail") {
        eprintln!("token=SECRET https://private.example/secret");
        std::process::exit(1);
    }
    let track = r#"{"id":"123","title":"Example","webpage_url":"https://soundcloud.com/a/b","duration":1.2346}"#;
    if target.starts_with("scsearch") {
        println!("{{\"entries\":[{track}]}}");
    } else {
        println!("{}", track.replace("}", ",\"url\":\"https://audio.example/temporary\"}"));
    }
}
