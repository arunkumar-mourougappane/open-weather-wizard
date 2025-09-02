#!/bin/bash

rm -Rf weather-icons

rm -fr ./assets/animated
rm -fr ./assets/static

# Clone the Makin-Things weather-icons repository
git clone --quiet https://github.com/Makin-Things/weather-icons.git weather-icons

# Create the assets directories
mkdir -p ./assets/animated
mkdir -p ./assets/static

# Copy animated icons
rsync -a --exclude 'README.md' weather-icons/animated/* ./assets/animated/

# If static icons exist, copy them too
if [ -d "weather-icons/static" ]; then
    rsync -a --exclude 'README.md' weather-icons/static/* ./assets/static/
fi
