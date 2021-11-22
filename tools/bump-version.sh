#!/usr/bin/env bash

FILES=("Cargo.toml" "version.txt")
old_tag=${1:1}
new_tag=${2:1}
for f in "${FILES[@]}"
do
   :
   sed -i "s/$old_tag/$new_tag/g" "$f"
done

git add -A
git commit -m "🚀 Bump version: $1 -> $2"
git tag "v$2"
git push --tags
git push