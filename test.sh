#!/bin/bash

# DECLARE VARIABLES FOR SETUP
POA_NETWORK=qan_poa
POA_IMAGE=qanplatform/poa:latest
RAFT_IMAGE=qanplatform/raft:latest
NUMBER_OF_NODES=3

# FUNCTION TO SILENTLY REMOVE CONTAINER
remove_container() {
    docker container stop $1 > /dev/null 2>&1
    docker container rm $1 > /dev/null 2>&1
    echo 'Removed container: ' $1
}

# IF 2nd PARAM PASSED AND IS A NUMBER
if [ -n "$2" ] && [ "$2" -eq "$2" ] 2>/dev/null; then

    # OVERRIDE DEFAULT NUMBER OF NODES
    NUMBER_OF_NODES=$2
fi

case $1 in

    # START TEST
    'start')

        # IF DOCKER IS INSTALLED AND EXECUTABLE
        if [ -x $(which docker) ]; then

            # PULL UP-TO-DATE IMAGES
            docker pull $POA_IMAGE
            docker pull $RAFT_IMAGE

            # IF NETWORK DOES NOT EXIST YET
            if ! docker network inspect $POA_NETWORK > /dev/null 2>&1; then

                # CREATE IT
                docker network create --driver bridge $POA_NETWORK > /dev/null 2>&1;
            fi

            # IF THERE IS A NETWORK CONTAINER
            if docker container inspect "qan_raft" > /dev/null 2>&1; then
                echo "Old network container qan_raft exists, removing..."
                # STOP AND REMOVE IT
                remove_container "qan_raft"
            fi

            # LAUNCH REQUIRED NUMBER OF NODES
            for ((i = 0 ; i < $NUMBER_OF_NODES ; i++)); do

                # IF THERE IS A CONTAINER ALREADY RUNNING
                if docker container inspect "qan_poa"$i > /dev/null 2>&1; then
                    echo "Old container qan_poa"$i "exists, removing..."
                    # STOP AND REMOVE IT
                    remove_container "qan_poa"$i
                fi
            done

            # START RAFT NETWORK
            docker container run --detach --name "qan_raft" $RAFT_IMAGE > /dev/null
            echo 'Started RAFT network'

            # CONNECT RAFT TO DOCKER NETWORK
            docker network connect $POA_NETWORK "qan_raft"

            # WAIT FOR RAFT STARTUP
            echo 'Waiting for RAFT network...'
            sleep 10

            # LAUNCH REQUIRED NUMBER OF NODES
            for ((i = 0 ; i < $NUMBER_OF_NODES ; i++)); do

                # DYNAMIC PORT FORWARDING FOR FIRST NODE
                PORTFORWARD=''
                if [ "$i" == "0" ]; then
                    PORTFORWARD='-p 8000:8000'
                fi

                # START NEW INSTANCES
                docker container create --name "qan_poa"$i $PORTFORWARD $POA_IMAGE -n=qan_raft:4222 > /dev/null 2>&1

                # CONNECT NODE TO DOCKER NETWORK
                docker network connect $POA_NETWORK "qan_poa"$i
                echo "Connected node qan_poa"$i "to the network"
            done

            # START DAEMON ON ALL NODES
            for ((i = 0 ; i < $NUMBER_OF_NODES ; i++)); do

                # START NEW INSTANCES
                docker container start "qan_poa"$i
                echo "Started node qan_poa"$i
            done
        fi
    ;;

    # STOP TEST
    'stop')

        # IF THERE IS A NETWORK CONTAINER
        if docker container inspect "qan_raft" > /dev/null 2>&1; then

            # STOP AND REMOVE IT
            remove_container "qan_raft"
        fi

        for ((i = 0 ; i < 100 ; i++)); do

            # IF THERE IS A CONTAINER ALREADY RUNNING
            if ! docker container inspect "qan_poa"$i > /dev/null 2>&1 ; then
                break
            fi

            # IF THERE IS A CONTAINER ALREADY RUNNING
            if docker container inspect "qan_poa"$i > /dev/null 2>&1 ; then

                # STOP AND REMOVE IT
                remove_container "qan_poa"$i
            fi
        done

        # REMOVE NETWORK AS WELL
        docker network rm $POA_NETWORK > /dev/null 2>&1
    ;;

    'tx')
        # IF THE RPC EXPOSED NODE HAS BEEN UP FOR AT LEAST A MINUTE
        if docker ps -a | grep qan_poa0 | grep 'Up' | grep 'minute' > /dev/null; then

            # START MAKING TRANSACTIONS
            for ((i = 0 ; i < 30 ; i++)); do
                echo "Sending transaction "$i
                docker container exec "qan_poa0" /usr/local/bin/maketx.sh
            done
        else

            # STILL WAITING FOR STARTUP
            echo "Node starting, please wait..."
        fi
    ;;

    # HELP
    *)
        echo 'Usage:'
        echo '========='
        echo 'START:'
        echo '    ./test.sh start $NUMBER_OF_NODES (default 3)'

        echo 'STOP:'
        echo '    ./test.sh stop'

        echo 'MAKE TXs:'
        echo '    ./test.sh tx'
    ;;
esac
