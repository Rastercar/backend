.PHONY: docker_run_deps docker_stop_deps run_dev

# [PROD-TODO] remove me
lazy:
	git add . && git commit -m "." && git push origin master

run_dev:
	IS_DEVELOPMENT=true AWS_PROFILE=rastercar-vitor cargo watch -x run

docker_run_deps: 
	docker-compose -f docker/docker-compose.yml -p rastercar_api up -d

docker_stop_deps:
	docker stop rastercar-db
	docker stop rastercar-rmq
	docker stop rastercar-jaeger