#!/bin/bash

set -o errexit -o nounset

project="sdf"
rev=$(git rev-parse --short HEAD)

cargo doc
cd target/doc
echo "<meta http-equiv=refresh content=0;url=${project}/index.html>" > index.html

git init

git remote add upstream git@github.com:gadomski/sdf-rs
git fetch upstream
git reset upstream/gh-pages

touch .

git add -A .
git commit -m "rebuild pages at ${rev}"
git push -q upstream HEAD:gh-pages
