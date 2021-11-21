#!/usr/bin/env bash

FILES=("CARGO.toml" "version.txt")
old_tag=${1:1}
new_tag=${2:1}
for f in "${FILES[@]}"
do
   :
   sed -i "s/$old_tag/$new_tag/g" "$f"
done

git add -A
git config --global user.email "releases@bitsy.ai"
git config --global user.name "Release Automation"
git commit -m "🚀 Bump version: $1 -> $2"
git push origin "$3"