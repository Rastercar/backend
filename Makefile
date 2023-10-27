# [PROD-TODO] remove me
.PHONY: lazy
lazy:
	git add . && git commit -m "." && git push origin master

.PHONY: run_dev
run_dev:
	RUST_LOG=info IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo watch -x run

.PHONY: run_debug
run_debug:
	RUST_LOG=debug IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo watch -x run

.PHONY: docker_run_deps
docker_run_deps: 
	docker compose -f docker/docker-compose.yml -p rastercar_api up -d

.PHONY: docker_stop_deps
docker_stop_deps:
	docker stop rastercar-db
	docker stop rastercar-rmq
	docker stop rastercar-jaeger