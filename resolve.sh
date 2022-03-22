#!/usr/bin/env bash

apps=$(occ app:list --output json | jq -r ".[] | keys[]")

for app in $apps
do
    app_path=$(occ app:getpath $app)

    integrity=$(occ integrity:check-app --output json $app)

    if [ $? -eq 0 ]; then
        continue
    fi

    to_delete=$(echo $integrity | jq -r '.EXTRA_FILE | keys[]')

    for f in $to_delete
    do
        p=$app_path/$f
        echo $p
        rm $p
    done
done
