use std::{
    collections::HashMap,
    net::{Shutdown, SocketAddr, TcpStream},
    time::Duration,
};

use lazy_static::lazy_static;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::ToSocketAddrs;

use crate::{error::AppError, llm::SlmRequest, types::Result, util::string};

#[derive(Deserialize, Serialize, Debug)]
enum DeviceType {
    Sensor,
    Actuator,
    Computer,
}

#[derive(Deserialize, Serialize, Debug)]
struct Device {
    id: &'static str,
    device_type: DeviceType,
    zone: &'static str,
    name: &'static str,
    description: &'static str,
}

impl Device {
    fn url(&self) -> String {
        format!("http://{}.local/js/hera/dev/HostSystem/invoke.rmi", self.id)
    }

    fn _json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|err| AppError::Serde(err))
    }
}

lazy_static! {
    static ref devices: Vec<Device> = vec![
        Device {
            id: "dht-sensor",
            device_type: DeviceType::Sensor,
            zone: "Kitchen",
            name: "DHT Sensor",
            description: "Sensor for temperature and humidity"
        },
        Device {
            id: "thermostat",
            device_type: DeviceType::Actuator,
            zone: "Kitchen",
            name: "Thermostat",
            description: "The controller for the central heating system"
        },
        Device {
            id: "thermostat-sensor",
            device_type: DeviceType::Sensor,
            zone: "Living Room",
            name: "Thermostat Sensor",
            description: "Temperature sensor for the central heating system"
        },
        Device {
            id: "hera",
            device_type: DeviceType::Computer,
            zone: "Living Room",
            name: "Mini PC",
            description: "Mini computer used as server for the home automation system"
        },
    ];
    static ref device_names: String = devices
        .iter()
        .map(|device| device.name)
        .collect::<Vec<&str>>()
        .join(", ");
}

fn device(name: &str) -> Option<&Device> {
    trace!("agent::hera::device(name: &str) -> Option<&Device>");
    let prompt = format!(
        r"Find device described by '{}' and return it's id.
Search in next devices: {}
Return only the device id. If device cannot be determined return 'none'.",
        name,
        ListDevices::new().exec().ok()?
    );
    debug!("prompt: {prompt}");
    let Ok(id) = SlmRequest::new(&prompt).exec() else {
        return None;
    };
    debug!("device id: {id}");
    devices.iter().find(|d| d.id == id.to_lowercase().trim())
}

fn device_not_found(name: &str) -> Result<String> {
    return Ok(format!(
        "device {} not found. registered devices: {}",
        name, *device_names
    ));
}

#[derive(Serialize, Debug)]
enum ValueType {
    Temperature,
    Humidity,
}

#[derive(Serialize, Debug)]
struct SensorValue {
    zone: String,
    name: String,
    value_type: ValueType,
    value: Value,
}

impl SensorValue {
    fn try_new(zone: &str, name: &str, value_type: ValueType, value: String) -> Result<Self> {
        let value: Value = serde_json::from_str(&value)?;
        Ok(Self {
            zone: zone.to_string(),
            name: name.to_string(),
            value_type,
            value,
        })
    }

    fn json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
}

#[derive(Serialize)]
struct DeviceAction {
    action: &'static str,
    description: &'static str,
}

lazy_static! {
    static ref dht_sensor_actions: Vec<DeviceAction> = vec![
        DeviceAction {
            action: "getHumidity",
            description: "Get the current humidity level."
        },
        DeviceAction {
            action: "getTemperature",
            description: "Get the current temperature value."
        },
        DeviceAction {
            action: "getHeatIndex",
            description: "Get the computed heat index."
        },
        DeviceAction {
            action: "getValue",
            description: "Get the sensor raw value."
        },
        DeviceAction {
            action: "getState",
            description: "Get device internal state."
        },
    ];
    static ref thermostat_actions: Vec<DeviceAction> = vec![
        DeviceAction {
            action: "updateSetpoint",
            description: "Get the current humidity level."
        },
        DeviceAction {
            action: "setSetpoint",
            description: "Set setpoint value."
        },
        DeviceAction {
            action: "getSetpoint",
            description: "Get setpoint value."
        },
        DeviceAction {
            action: "setTemperature",
            description: "Set the current temperature value."
        },
        DeviceAction {
            action: "getTemperature",
            description: "Get the current temperature value."
        },
        DeviceAction {
            action: "update",
            description: "Update running state based on setpoint and current temperature."
        },
        DeviceAction {
            action: "getState",
            description: "Get device internal state."
        },
    ];
    static ref thermostat_sensor_actions: Vec<DeviceAction> = vec![
        DeviceAction {
            action: "getValue",
            description: "Get cached temperature value."
        },
        DeviceAction {
            action: "readValue",
            description: "Get the sensor temperature value."
        },
    ];
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ListDevices {}

impl ListDevices {
    fn new() -> Self {
        Self {}
    }

    pub fn exec(&self) -> Result<String> {
        trace!("agent::hera::ListDevices::exec(&self) -> Result<String>");
        Ok(devices
            .iter()
            .map(|device| serde_json::to_string(device).unwrap())
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DescribeDevice {
    device: String,
}

impl DescribeDevice {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::DescribeDevice::exec(&self) -> Result<String>");
        debug!("device: {}", self.device);

        let Some(device) = device(&self.device) else {
            return device_not_found(&self.device);
        };

        let mut description = String::new();
        description.push_str(&format!("Device: {}\n", serde_json::to_string(&device)?));

        if let Ok(response) = reqwest::Client::new()
            .post(device.url())
            .body(format!(r#"["{}", "getActions", ""]"#, self.device))
            .send()
            .await
        {
            if response.status().is_success() {
                // ["updateSetpoint","setSetpoint","getSetpoint","setTemperature","getTemperature","update","getState"]
                description.push_str(&format!("Actions: {}\n", response.text().await?));
            }
        }
        Ok(description)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetDeviceActions {
    device: String,
}

impl GetDeviceActions {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::GetDeviceActions::exec(&self) -> Result<String>");
        debug!("device: {}", self.device);

        let Some(device) = device(&self.device) else {
            return device_not_found(&self.device);
        };

        let actions: &Vec<DeviceAction> = match device.id {
            "dht-sensor" => &dht_sensor_actions,
            "thermostat" => &thermostat_actions,
            "thermostat-sensor" => &thermostat_sensor_actions,
            _ => {
                return Err(AppError::Fatal("".to_string()));
            }
        };

        Ok(actions
            .iter()
            .map(|a| serde_json::to_string(a).unwrap())
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReadTemperature {
    zone: String,
}

impl ReadTemperature {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::ReadTemperature::exec(&self) -> Result<String>");
        debug!("zone: {}", self.zone);

        let Some(device) = devices
            .iter()
            .find(|d| string::eq_no_case(&d.zone, &self.zone))
        else {
            return Ok(r#"{"error":"device not found"}"#.to_string());
        };
        let action = match self.zone.to_lowercase().as_str() {
            "kitchen" => "getTemperature",
            "living room" => "getValue",
            _ => return Ok(r#"{"error":"no temperature sensor in zone"}"#.to_string()),
        };
        let value = reqwest::Client::new()
            .post(device.url())
            .body(format!(r#"["{}", "{action}", ""]"#, &device.id))
            .send()
            .await?
            .text()
            .await?;
        // {"zone":"Kitchen","name":"DHT Sensor","value_type":"Temperature","value":17.5}
        SensorValue::try_new(&device.zone, &device.name, ValueType::Temperature, value)?.json()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReadHumidity {}

impl ReadHumidity {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::ReadHumidity::exec(&self) -> Result<String>");

        let Some(device) = devices.iter().find(|d| d.id == "dht-sensor") else {
            return Ok(r#"{"error":"device not found"}"#.to_string());
        };
        let value = reqwest::Client::new()
            .post(device.url())
            .body(format!(r#"["{}", "getHumidity", ""]"#, &device.id))
            .send()
            .await?
            .text()
            .await?;
        // {"zone":"Kitchen","name":"DHT Sensor","value_type":"Humidity","value":16.3}
        SensorValue::try_new(&device.zone, &device.name, ValueType::Humidity, value)?.json()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReadSensors {}

impl ReadSensors {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::ReadSensors::exec(&self) -> Result<String>");

        let mut result = String::new();
        for (sensor, measurement, action) in [
            ("dht-sensor", ValueType::Humidity, "getHumidity"),
            ("dht-sensor", ValueType::Temperature, "getTemperature"),
            ("thermostat-sensor", ValueType::Temperature, "getValue"),
        ] {
            let device = device(sensor).unwrap();
            let value = reqwest::Client::new()
                .post(device.url())
                .body(format!(r#"["{}", "{action}", ""]"#, &device.id))
                .send()
                .await?
                .text()
                .await?;
            let value = SensorValue::try_new(&device.zone, &device.name, measurement, value)?;
            result.push_str(&serde_json::to_string(&value)?);
            result.push('\n');
        }
        // {"zone":"Kitchen","name":"DHT Sensor","value_type":"Humidity","value":15.2}
        // {"zone":"Kitchen","name":"DHT Sensor","value_type":"Temperature","value":17.5}
        // {"zone":"Living Room","name":"Thermostat Sensor","value_type":"Temperature","value":21.38}
        Ok(result)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StartHeating {}

impl StartHeating {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::StartHeating::exec(&self) -> Result<String>");
        let response = reqwest::Client::new()
            .post("http://thermostat.local/js/hera/dev/HostSystem/invoke.rmi")
            .body(r#"["thermostat", "updateSetpoint", "30"]"#)
            .send()
            .await?;
        // {"setpoint": 10.00,"hysteresis": 0.00,"temperature": 19.94,"running": false}
        Ok(response.text().await?)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StopHeating {}

impl StopHeating {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::StopHeating::exec(&self) -> Result<String>");
        let response = reqwest::Client::new()
            .post("http://thermostat.local/js/hera/dev/HostSystem/invoke.rmi")
            .body(r#"["thermostat", "updateSetpoint", "10"]"#)
            .send()
            .await?;
        // {"setpoint": 10.00,"hysteresis": 0.00,"temperature": 19.94,"running": false}
        Ok(response.text().await?)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetHeatingState {}

impl GetHeatingState {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::GetHeatingState::exec(&self) -> Result<String>");
        let response = reqwest::Client::new()
            .post("http://thermostat.local/js/hera/dev/HostSystem/invoke.rmi")
            .body(r#"["thermostat", "getState", ""]"#)
            .send()
            .await?;
        // {"setpoint": 10.00,"hysteresis": 0.00,"temperature": 19.94,"running": false}
        Ok(response.text().await?)
    }
}

#[derive(Deserialize, Debug)]
pub struct RunDiagnose {
    device: String,
}

impl RunDiagnose {
    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::RunDiagnose::exec(&self) -> Result<String>");
        if self.device.eq_ignore_ascii_case("system") {
            RunSystemDiagnose::new().exec().await
        } else {
            RunDeviceDiagnose::new(&self.device).exec().await
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RunDeviceDiagnose {
    device: String,
}

impl RunDeviceDiagnose {
    fn new(device: &str) -> Self {
        Self {
            device: device.to_string(),
        }
    }

    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::RunDeviceDiagnose::exec(&self) -> Result<String>");
        debug!("device: {}", self.device);

        let Some(device) = device(&self.device) else {
            return device_not_found(&self.device);
        };

        let hostname = &format!("{}.local", device.id);
        let port = 80;
        debug!("hostname: {hostname}, port: {port}");

        let Some(addr) = (hostname.clone(), port).to_socket_addrs()?.next() else {
            return Ok(format!("cannot resolve {hostname}"));
        };
        let Ok(socket) = TcpStream::connect_timeout(&addr, Duration::from_secs(20)) else {
            return Ok(format!("cannot connect device on port 80"));
        };
        let peer_addr = socket.peer_addr()?;
        debug!("peer_addr: {peer_addr}");
        socket.shutdown(Shutdown::Write)?;

        let mut report = DiagnoseReport::new(&device, &peer_addr, hostname);
        for action in ["getState", "getValue"] {
            debug!("action: {action}");
            let response = reqwest::Client::new()
                .post(device.url())
                .body(format!(r#"["{}", "{}", ""]"#, device.id, action))
                .send()
                .await?;
            if response.status().is_success() {
                let response = response.text().await?;
                if !response.starts_with("Action not found") {
                    // {"setpoint": 10.00,"hysteresis": 0.00,"temperature": 19.94,"running": false}
                    report.set_device_state(&response)?;
                    break;
                }
            }
        }

        debug!("report: {report:?}");
        return report.json();
    }
}

#[derive(Deserialize, Serialize, Debug)]
enum ConnectionState {
    Active,
    Fail,
}

#[derive(Deserialize, Serialize, Debug)]
struct DiagnoseReport {
    device_id: String,
    device_name: String,
    device_type: String,
    device_description: String,
    device_zone: String,
    hostname: String,
    ipv4_addr: String,
    diagnose_port: u16,
    http_request_url: String,
    connection_state: ConnectionState,
    device_state: HashMap<String, String>,
}

impl DiagnoseReport {
    fn new(device: &Device, socket: &SocketAddr, hostname: &str) -> Self {
        Self {
            device_id: device.id.to_string(),
            device_name: device.name.to_string(),
            device_type: format!("{:?}", device.device_type),
            device_description: device.description.to_string(),
            device_zone: device.zone.to_string(),
            hostname: hostname.to_string(),
            ipv4_addr: socket.ip().to_string(),
            diagnose_port: socket.port(),
            http_request_url: device.url(),
            connection_state: ConnectionState::Active,
            device_state: HashMap::new(),
        }
    }

    fn set_device_state(&mut self, response: &str) -> Result<()> {
        let value: Value = serde_json::from_str(response)?;
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    self.device_state.insert(key, value.to_string());
                }
            }
            value => {
                let key = "value".to_string();
                self.device_state.insert(key, value.to_string());
            }
        }
        Ok(())
    }

    fn json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|err| AppError::Serde(err))
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RunSystemDiagnose {
    level: u8,
}

impl RunSystemDiagnose {
    fn new() -> Self {
        trace!("agent::hera::RunSystemDiagnose::new() -> Self");
        Self { level: 10 }
    }

    pub async fn exec(&self) -> Result<String> {
        trace!("agent::hera::RunSystemDiagnose::exec(&self) -> Result<String>");
        debug!("level: {}", self.level);
        let mut report: Vec<String> = Vec::new();
        for device in devices.iter() {
            let diagnose = RunDeviceDiagnose::new(&device.id);
            let result = diagnose.exec().await.unwrap_or_else(|e| e.to_string());
            report.push(result);
        }
        Ok(report.join("\n"))
    }
}
