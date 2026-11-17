fn main() {
    let target = std::env::args().last().unwrap();
    if target.contains("hang") {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
    if target.contains("fail") {
        eprintln!("token=SECRET");
        std::process::exit(1);
    }
    if target.starts_with("scsearch") {
        if target.contains("empty") {
            println!("{{\"entries\":[]}}");
        } else {
            println!(
                "{{\"entries\":[{{\"id\":\"1\",\"title\":\"Example\",\"uploader\":\"Artist\",\"webpage_url\":\"https://soundcloud.com/a/b\"}}]}}"
            );
        }
    } else if target.starts_with("https://soundcloud.com") {
        let stream = if target.ends_with("/unplayable") {
            "fail"
        } else {
            "stream"
        };
        println!(
            "{{\"id\":\"1\",\"title\":\"Example\",\"webpage_url\":\"https://soundcloud.com/a/b\",\"url\":\"https://audio.example/{stream}\"}}"
        );
    } else {
        println!("FUZZY_PLAYING");
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
