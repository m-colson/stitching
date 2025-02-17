# Stitching Server
Server and Website to display live projected video.

## Client-Server Protocol
Uses a websocket at */video* with the following binary protocol:

#### Packet Type (1 byte)
| Name              | Value  |
|:----------------- | ------:|
| NOP               |      0 |
| Settings Sync     |      1 |
| Update Frame      |      2 |
| Update Detections |      3 |

### Settings Sync
| Field         | Type |
|:------------- |:---- |
| view_type     | u8   |

### Update Frame
| Field         | Type                                  |
|:------------- |:------------------------------------- |
| width         | u16                                   |
| height        | u16                                   |
| bytes_per_pix | u8                                    |
| __reserved    | *3 bytes*                             |
| send_millis   | f64                                   |
| data          | \[width * height * bytes_per_pix\] u8 |

### Update Detections
| Field         | Type                  |
|:------------- |:--------------------- |
| num_dt        | u16                   |
| bounds        | \[num_dt\] Detection  |

## Types
### Detection
| Field         | Type                  |
|:------------- |:--------------------- |
| obj_class     | u8                    |
| conf_decimal  | u8                    |
| azimuth       | u16                   |
