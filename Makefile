IMAGE?=caffalaughrey/focloireacht-server
TAG?=latest
PLATFORM?=linux/amd64

.PHONY: build run push

build:
	DOCKER_BUILDKIT=1 docker build --platform $(PLATFORM) -t $(IMAGE):$(TAG) .

run:
	docker run --rm -p 5005:5005 \
	  -e BIND_ADDR=0.0.0.0:5005 \
	  -v $$PWD/vendor/irish-lex-db:/data:ro \
	  $(IMAGE):$(TAG)

push:
	docker push $(IMAGE):$(TAG)

# Example docker-compose service (join existing external network `gateway_net`):
# services:
#   focloireacht:
#     image: $(IMAGE):$(TAG)
#     environment:
#       - BIND_ADDR=0.0.0.0:5005
#       - LEX_DB_PATH=/data/lexicon.sqlite
#       - TERM_DB_PATH=/data/terminology.sqlite
#     volumes:
#       - ./vendor/irish-lex-db:/data:ro
#     ports:
#       - "5005:5005"
#     networks:
#       - gateway_net
# networks:
#   gateway_net:
#     external: true
