.PHONY: docker_run_deps docker_stop_deps run_dev

run_dev:
	IS_DEVELOPMENT=true cargo watch -x run

docker_run_deps: 
	docker-compose -f docker/docker-compose.yml -p rastercar_api up -d

docker_stop_deps:
	docker stop rastercar-db
	docker stop rastercar-rmq
	docker stop rastercar-jaeger