#!/usr/bin/env bash

apps=$(occ app:list --output json | jq -r ".[] | keys[]")

for app in $apps
do
    echo "Checking $app"

    to_delete=$(occ integrity:check-app --output json "$app" | jq -r 'if type=="array" then [] else (.EXTRA_FILE | keys[]) end')

    app_path=$(occ app:getpath "$app")

    for f in $to_delete
    do
        p=$app_path/$f
        printf "\t %b\n" "$p"
        rm "$p"
    done
done
