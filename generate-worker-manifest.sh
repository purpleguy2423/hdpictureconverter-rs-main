#!/bin/sh
#
# Generate a "worker manifest" for service worker pre-cache.

if [ $# -eq 0 ] || [ ! -d "$1" ]
then
    echo "Usage: $0 <content_dir>"
    exit 1
fi

manifest=worker-cache-manifest.js
out="$1/$manifest"
files=$(find "$1" -type f -not -name "$manifest" | sort)

echo "const cachePrefix = 'hdpictureconverter-rs-';" >> $out
cat $files | sha1sum | awk '{ print "const cacheName = cachePrefix + '\''"$1"'\'';"; }' >> $out
echo "const cacheFiles = [" >> $out
# / should point to index.html, but doesn't exist as a file
echo "    '/'," >> $out
for f in $files
do
    echo "    '$f'," >> $out
done
echo "];" >> $out
