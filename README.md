# Fuzzy Music

ВУЗовский проект по ВПД (введение в профессиональную деятельность).

Музыкальный плеер для Windows и Linux. Планируемый стек: Rust, Tauri 2, React и TypeScript.

Сейчас реализован самостоятельный Rust-модуль проверки SoundCloud. Он умеет искать публичные треки и получать доступный аудиопоток. Графический интерфейс для проверки не нужен.

## Быстрый запуск на Windows x64

Необходимы Git, Rust stable и Microsoft C++ Build Tools с Windows SDK.

```powershell
.\scripts\check-soundcloud.ps1 -Query 'Молчат Дома' -Limit 3
```

При первом запуске скрипт скачает официальный yt-dlp в локальную папку tools и проверит SHA-256. SoundCloud-аккаунт, подписка и ручной API-ключ не требуются. Нужен доступ к SoundCloud и серверам загрузки зависимостей.

Ожидаемый результат: JSON со списком треков — названием, автором загрузки, продолжительностью, обложкой и ссылкой SoundCloud. Порядок и состав выдачи могут меняться.

```powershell
.\scripts\check-soundcloud.ps1 -Mode resolve -Query 'https://soundcloud.com/hephestosmusic/super-mario-world-game-over'
```

Команда resolve выводит сведения о доступном потоке. Она не запускает звук и не сохраняет полный трек.

## Проверки кода

```text
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

Подробности: [разработка и самостоятельная проверка модуля SoundCloud](docs/soundcloud-development.md).
