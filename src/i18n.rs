use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Serialize)]
pub enum Language {
    Russian,
    #[default]
    English,
}

macro_rules! translations {
    ($($(#[$attribute:meta])* $key:ident => $ru:literal, $en:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Key { $($(#[$attribute])* $key,)* }

        impl Language {
            pub fn text(self, key: Key) -> &'static str {
                match key {
                    $($(#[$attribute])* Key::$key => match self {
                        Self::Russian => $ru,
                        Self::English => $en,
                    },)*
                }
            }
        }
    };
}

translations! {
    Settings => "Настройки", "Settings";
    #[cfg(windows)]
    OpenWindow => "Открыть", "Open";
    #[cfg(windows)]
    Exit => "Выход", "Exit";
    Language => "Язык интерфейса", "Interface language";
    Done => "Готово", "Done";
    ScreenLighting => "Подсветка экрана", "Screen lighting";
    Devices => "Устройства", "Devices";
    NoDevices => "Устройств пока нет", "No devices yet";
    AddDevice => "Добавить устройство", "Add device";
    Add => "Добавить", "Add";
    Added => "Добавлено", "Added";
    Cancel => "Отмена", "Cancel";
    Address => "IPv4-адрес", "IPv4 address";
    AlreadyAdded => "Устройство с таким адресом уже добавлено", "A device with this address has already been added";
    InvalidAddress => "Введите корректный IPv4-адрес", "Enter a valid IPv4 address";
    AutomaticSearch => "Автоматический поиск", "Automatic search";
    NoAddresses => "Адреса не найдены.", "No addresses found.";
    SearchFailed => "Не удалось запустить поиск.", "Could not start the search.";
    Checking => "Проверка…", "Checking…";
    Online => "В сети", "Online";
    NoReply => "Нет ответа", "No response";
    Sync => "Синхронизация", "Synchronization";
    SyncRunning => "Идёт синхронизация", "Sync running";
    SyncOff => "Синхронизация выключена", "Sync is off";
    Start => "Запустить", "Start";
    Stop => "Остановить", "Stop";
    Stopping => "Остановка…", "Stopping…";
    StopFirst => "Сначала остановите синхронизацию.", "Stop synchronization first.";
    AddDeviceHelp => "Добавьте устройство, чтобы настроить подсветку.", "Add a device to set up screen lighting.";
    SetupHelp => "Выберите устройство, настройте монитор и ленту, затем запустите синхронизацию.", "Select a device, configure its monitor and LED strip, then start synchronization.";
    DeviceSettings => "Настройки устройства", "Device settings";
    DeviceSettingsFor => "Настройки устройства {value}", "Settings for device {value}";
    RemoveDevice => "Удалить устройство", "Remove device";
    ConfirmRemoval => "Удалить устройство?", "Remove device?";
    RemovalDetails => "Устройство {value} и его настройки будут удалены.", "Device {value} and its settings will be removed.";
    Remove => "Удалить", "Remove";
    Capture => "Захват экрана", "Screen capture";
    CaptureArea => "Область захвата", "Capture area";
    Monitor => "Монитор", "Monitor";
    SelectMonitor => "Выберите монитор", "Select a monitor";
    AvailableMonitor => "Выберите доступный монитор.", "Select an available monitor.";
    MonitorBusy => "Монитор уже используется устройством {value}.", "This monitor is already in use by device {value}.";
    PrimaryMonitor => "основной", "primary";
    BandDepth => "Глубина полосы, %", "Band depth, %";
    FrameRate => "Кадров в секунду", "Frames per second";
    StripLayout => "Расположение ленты", "LED strip layout";
    WiringOrder => "Порядок подключения от первого светодиода.", "Wiring order, starting from the first LED.";
    StripNotConfigured => "Лента не настроена", "LED strip not configured";
    LedTotal => "Всего {value} LED", "Total: {value} LEDs";
    Top => "Верхняя", "Top";
    Right => "Правая", "Right";
    Bottom => "Нижняя", "Bottom";
    Left => "Левая", "Left";
    Reverse => "Реверс", "Reverse";
    ReverseDirection => "Обратное направление ленты", "Reverse the strip direction";
    MoveUp => "Переместить выше", "Move up";
    MoveDown => "Переместить ниже", "Move down";
    LedCountRequired => "Укажите число LED на каждой стороне.", "Enter the LED count for each side.";
    LedLimit => "Всего допускается до 450 LED.", "The total must be 450 LEDs or fewer.";
    FrameRateRange => "Частота должна быть от 1 до 60 FPS.", "Frame rate must be between 1 and 60 FPS.";
    BandDepthRange => "Глубина полосы должна быть от 1 до 50%.", "Band depth must be between 1 and 50%.";
    #[cfg(not(windows))]
    WindowsOnly => "Захват экрана доступен только в Windows.", "Screen capture is available only on Windows.";
    CaptureThreadFailed => "Поток захвата завершился с ошибкой.", "The capture thread failed.";
    MonitorUnavailable => "Выбранный монитор недоступен.", "The selected monitor is unavailable.";
    MonitorListFailed => "Не удалось получить список мониторов.", "Could not list monitors.";
    SocketFailed => "Не удалось открыть UDP-сокет.", "Could not open the UDP socket.";
    TargetFailed => "Не удалось выбрать адрес ленты.", "Could not set the LED strip address.";
    CaptureStartFailed => "Не удалось запустить захват экрана.", "Could not start screen capture.";
    CaptureStopFailed => "Ошибка завершения захвата.", "Could not stop screen capture.";
    CaptureEnded => "Захват экрана прекратился.", "Screen capture stopped unexpectedly.";
    MonitorDisconnected => "Монитор отключён. Выберите источник и запустите синхронизацию заново.", "The monitor was disconnected. Select a source and restart synchronization.";
    FrameSendFailed => "Не удалось отправить кадр на ленту.", "Could not send the frame to the LED strip.";
    PartialFrame => "UDP-кадр отправлен не полностью.", "The UDP frame was not sent in full.";
    FirstFrameMissing => "Не удалось получить первый кадр экрана.", "Could not get the first screen frame.";
    EmptyFrame => "Пустой кадр экрана.", "Empty screen frame.";
    FrameTooLarge => "Слишком большой кадр.", "Screen frame is too large.";
    InvalidBgra => "Некорректный размер BGRA-кадра.", "Invalid BGRA frame size.";
    InvalidRgb => "Некорректный размер RGB-буфера.", "Invalid RGB buffer size.";
}

impl Language {
    pub fn format(self, key: Key, value: impl Display) -> String {
        self.text(key).replace("{value}", &value.to_string())
    }

    pub fn error(self, error: &anyhow::Error) -> String {
        match error.downcast_ref::<Key>() {
            Some(&key) => match error.source() {
                Some(source) if source.downcast_ref::<Key>() != Some(&key) => {
                    let root = error.root_cause();
                    let cause = root
                        .downcast_ref::<Key>()
                        .map_or_else(|| root.to_string(), |&key| self.text(key).to_owned());
                    format!("{} {cause}", self.text(key))
                }
                _ => self.text(key).to_owned(),
            },
            None => error.to_string(),
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Language::English.text(*self))
    }
}

impl std::error::Error for Key {}
