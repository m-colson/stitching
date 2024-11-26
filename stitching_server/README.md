# Stitching Server
Server and Website to display live projected video.

## Client-Server Protocol
Uses a websocket at */video* with the following binary protocol:

#### Packet Type (1 byte)
| Name          | Value  |
|:------------- | ------:|
| NOP           |      0 |
| Settings Sync |      1 |
| Update Frame  |      2 |
| Update Bounds |      3 |

### Settings Sync
| Field         | Type |
|:------------- |:---- |
| view_type     | u8   |

### Update Frame
| Field         | Type                                |
|:------------- |:----------------------------------- |
| width         | u16                                 |
| height        | u16                                 |
| bytes_per_pix | u8                                  |
| __reserved    | *2 bytes*                           |
| data          | [width * height * bytes_per_pix] u8 |

### Update Bounds
| Field         | Type                  |
|:------------- |:--------------------- |
| num_bounds    | u16                   |
| bounds        | [num_bounds] BoundBox |

#### BoundBox
| Field    | Type |
|:-------- |:---- |
| x        | u16  |
| y        | u16  |
| width    | u16  |
| height   | u16  |
| class_id | u8   |