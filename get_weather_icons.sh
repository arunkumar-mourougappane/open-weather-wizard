#!/bin/bash

rm -Rf weather-icons

rm -fr ./assets/animated
rm -fr ./assets/static

git clone --quiet https://github.com/bramkragten/weather-card.git weather-icons --branch v1.5.0

rsync -a --exclude 'README.md' weather-icons/icons/* ./assets/
