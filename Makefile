.PHONY: lazy
lazy:
	git add . && git commit -m "." && git push origin master
	
# ---------------------- [API] ----------------------
.PHONY: run_api_dev
run_api_dev:
	RUST_LOG=warn,sea_orm=debug,sqlx_logging=debug IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p api'

.PHONY: run_api_dev_no_watch
run_api_dev_no_watch:
	RUST_LOG=warn,sea_orm=debug,sqlx_logging=debug IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo run -p api
	
.PHONY: run_api_error
run_api_error:
	RUST_LOG=error AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p api'

.PHONY: run_api_warn
run_api_warn:
	RUST_LOG=warn AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p api'

.PHONY: run_api_info
run_api_info:
	RUST_LOG=info IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p api' 

.PHONY: run_api_debug
run_api_debug:
	RUST_LOG=debug IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p api'

# ---------------------- [MAILER] ----------------------
.PHONY: run_mailer_dev
run_mailer_dev:
	RUST_LOG=info AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p mailer'

.PHONY: run_mailer_dev_no_watch
run_mailer_dev_no_watch:
	RUST_LOG=info AWS_PROFILE=rastercar-vitor cargo run -p mailer
	
.PHONY: run_mailer_error
run_mailer_error:
	RUST_LOG=error AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p mailer'

.PHONY: run_mailer_warn
run_mailer_warn:
	RUST_LOG=warn AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p mailer'

.PHONY: run_mailer_info
run_mailer_info:
	RUST_LOG=info AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p mailer' 

.PHONY: run_mailer_debug
run_mailer_debug:
	APP_DEBUG=true RUST_LOG=debug AWS_PROFILE=rastercar-vitor cargo watch -x 'run -p mailer'

# ---------------------- [DECODER] ----------------------
.PHONY: run_decoder_dev
run_decoder_dev:
	RUST_LOG=warn cargo watch -x 'run -p decoder'

.PHONY: run_decoder_dev_no_watch
run_decoder_dev_no_watch:
	RUST_LOG=warn cargo run -p decoder
	
.PHONY: run_decoder_error
run_decoder_error:
	RUST_LOG=error cargo watch -x 'run -p decoder'

.PHONY: run_decoder_warn
run_decoder_warn:
	RUST_LOG=warn cargo watch -x 'run -p decoder'

.PHONY: run_decoder_info
run_decoder_info:
	RUST_LOG=info cargo watch -x 'run -p decoder' 

.PHONY: run_decoder_debug
run_decoder_debug:
	RUST_LOG=debug cargo watch -x 'run -p decoder'

# ---------------------- [API] ----------------------
.PHONY: docker_run_deps
docker_run_deps: 
	docker compose -f docker/docker-compose.yml -p rastercar_api up -d

.PHONY: docker_stop_deps
docker_stop_deps:
	docker stop rastercar-db
	docker stop rastercar-rmq
	docker stop rastercar-jaeger
