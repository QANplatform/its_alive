#!/bin/bash

for i in $(docker container ls -a | grep kumar | cut -d' ' -f 1); do
	docker stop "$i"
	docker container rm "$i"
done

if [[ $1 -ne 0 ]]; then
docker run -i --net=subs --name="kumar0" -h "kumar0" -d --ip="172.33.0.2" poademo
fi

for i in $(seq 1 $1); do
	docker run -i --net=subs --name="kumar$i" -d poademo
done