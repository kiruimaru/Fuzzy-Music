# Запуск и проверка SoundCloud

## Требования

- Windows x64: Rust stable, Microsoft C++ Build Tools и Windows SDK.
- Linux x86_64: Rust stable и системные инструменты компиляции.
- Интернет для загрузки зависимостей и обращения к SoundCloud.

Аккаунт SoundCloud, подписка и ручной API-ключ не требуются. Команды выполняются из корня репозитория.

## Поиск на Windows

```powershell
.\scripts\check-soundcloud.ps1 -Query 'Молчат Дома' -Limit 3
```

При первом запуске скрипт скачивает yt-dlp и проверяет SHA-256.

Результат — JSON со списком треков: название, автор загрузки, длительность, обложка и ссылка SoundCloud. Некоторые поля могут быть null. Пустой список допустим; состав и порядок выдачи могут меняться.

## Получение потока

Передайте ссылку webpage_url из результата поиска:

```powershell
.\scripts\check-soundcloud.ps1 -Mode resolve -Query 'https://soundcloud.com/hephestosmusic/super-mario-world-game-over'
```

Результат — JSON с данными трека, временным адресом потока и форматом. Команда не запускает звук и не скачивает полный трек. Для новой попытки воспроизведения адрес следует получать заново.

Отдельные записи могут быть доступны только как превью или недоступны для воспроизведения.

## Тайм-аут и отмена

Тайм-аут запроса по умолчанию — 45 секунд. Для проверки остановки:

```powershell
.\scripts\check-soundcloud.ps1 -Query 'lofi hip hop' -TimeoutSeconds 1
.\scripts\check-soundcloud.ps1 -Query 'lofi hip hop' -CancelAfterSeconds 0.25
```

Если запрос ещё выполняется, ожидаются сообщение timed out или cancelled и код завершения 1. Если он успел закончиться раньше, возвращается результат. Время загрузки инструментов и сборки не входит в тайм-аут. Параметр CancelAfterSeconds демонстрирует программную отмену и не обрабатывает Ctrl+C.

## Проверки кода

```text
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

Тесты не обращаются к SoundCloud. Для проверки соединения выполните реальный поиск.

## Linux x86_64

```bash
mkdir -p tools
curl --fail --location --output tools/yt-dlp_linux https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp_linux
printf '%s  %s\n' '767ba9dce7052aa196f8b12652f433d76de128ba168966fa84b6394d5e6c6795' 'tools/yt-dlp_linux' | sha256sum --check
# Продолжайте только при успешной проверке SHA-256.
chmod +x tools/yt-dlp_linux
cargo run --locked -p fuzzy-soundcloud --bin soundcloud-probe -- search 'lofi hip hop' --limit 5 --yt-dlp ./tools/yt-dlp_linux
```

Запуск проверен на Windows. Проверка Linux пока не выполнена.
