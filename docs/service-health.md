# Service — Health Module

The `service::health` module persists and retrieves personal health measurements in MongoDB. It is the data layer backing `HealthAgent`.

## Key Characteristics

| Property | Value |
|---|---|
| Database | MongoDB at `mongodb://localhost:27017` |
| Database name | `jarvis` |
| Collection | `health` |
| Record model | `{ timestamp, date, person, measurement, value, units }` |

## Supported Operations

| Struct | Description | Measurements written |
|---|---|---|
| `SaveBlood` | Saves a blood pressure reading | `systolic_pressure`, `diastolic_pressure`, `pulse_pressure`, `heart_rate` |
| `SaveTemperature` | Saves a body temperature reading | `body_temperature` |
| `SaveWeight` | Saves body weight; computes BMI automatically | `body_weight`, `body_mass_index` |
| `SaveGlucose` | Saves a blood glucose level | `glucose_level` |
| `ReadMeasurements` | Queries all measurements for a person on a given date | *(read only)* |

## Class Diagram

```mermaid
classDiagram
    class Archive {
        -collection : Collection~Record~
        +new()$ Result~Archive~
        -save_measurement(person, measurement, value) Result~Measurement~
        -read_measurements(person, date) Result~Vec~Measurement~~
    }

    class SaveBlood {
        <<struct>>
        +timestamp : DateTime
        +person    : String
        +systole   : u32
        +diastole  : u32
        +pulse     : u32
        +exec() Result~String~
    }

    class SaveTemperature {
        <<struct>>
        +timestamp   : DateTime
        +person      : String
        +temperature : f32
        +exec() Result~String~
    }

    class SaveWeight {
        <<struct>>
        +timestamp        : DateTime
        +person           : String
        +height           : f32
        +weight           : f32
        +body_mass_index  : Option~f32~
        +exec() Result~String~
    }

    class SaveGlucose {
        <<struct>>
        +timestamp : DateTime
        +person    : String
        +glucose   : f32
        +exec() Result~String~
    }

    class ReadMeasurements {
        <<struct>>
        +person : String
        +date   : String
        +exec() Result~String~
    }

    SaveBlood        --> Archive : uses
    SaveTemperature  --> Archive : uses
    SaveWeight       --> Archive : uses
    SaveGlucose      --> Archive : uses
    ReadMeasurements --> Archive : uses
```

## Sequence Diagrams

### Save Measurement

```mermaid
sequenceDiagram
    participant Agent as HealthAgent
    participant Op as SaveBlood / SaveTemperature / SaveWeight / SaveGlucose
    participant Archive
    participant DB as MongoDB

    Agent->>Op: exec()
    Op->>Archive: new()
    Archive->>DB: connect mongodb://localhost:27017
    loop for each measurement key
        Op->>Archive: save_measurement(person, key, value)
        Archive->>DB: insert_one(Record)
        DB-->>Archive: ok
        Archive-->>Op: Measurement
    end
    Op-->>Agent: JSON string(s)
```

### Read Measurements

```mermaid
sequenceDiagram
    participant Agent as HealthAgent
    participant Op as ReadMeasurements
    participant Archive
    participant DB as MongoDB

    Agent->>Op: exec()
    Op->>Archive: new()
    Archive->>DB: connect mongodb://localhost:27017
    Op->>Archive: read_measurements(person, date)
    Archive->>DB: find({ person, date })
    DB-->>Archive: cursor
    loop for each record
        Archive-->>Op: Measurement
    end
    Op-->>Agent: newline-separated JSON strings
```

## Measurement Units

| Measurement key | Unit |
|---|---|
| `systolic_pressure` | mmHg |
| `diastolic_pressure` | mmHg |
| `pulse_pressure` | mmHg |
| `heart_rate` | bpm |
| `glucose_level` | mg/dL |
| `body_temperature` | °C |
| `body_weight` | kg |
| `body_mass_index` | kg/m² |

## BMI Calculation

`SaveWeight` normalises the height to metres before computing BMI (in case the caller supplies it in centimetres) and stores both `body_weight` and `body_mass_index` in a single transaction.
