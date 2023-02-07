build:
	DOCKER_BUILDKIT=1 docker build . -t arso_exporter:local --no-cache
