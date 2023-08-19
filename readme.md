# Rastercar API

```bash
cargo watch -x run

diesel migration run --database-url=postgres://raster_user:raster_pass@localhost/raster_dev

diesel print-schema > src/database/schema.rs --database-url=postgres://raster_user:raster_pass@localhost/raster_dev

diesel_ext --model > src/models.rs
```
