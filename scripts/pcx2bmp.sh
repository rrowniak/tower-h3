#!/bin/bash -e 

RES_PROG=../src/target/debug/h3res
if [ ! -f $RES_PROG ]; then
    echo "Program $RES_PROG not found. Try building the workspace"
    exit 1
fi

io_dir="/home/rr/Games/H3Dump/Data"

for pcx_file in "$io_dir"/*.pcx; do
  #pcx_file="${io_dir}/TpThBRrm.pcx"
  if [ -f "$pcx_file" ]; then
    echo "Converting $pcx_file..."
    $RES_PROG pcx2bmp "$pcx_file" "${pcx_file}.bmp"
    echo "$lod_file converted successfully!"
  else
    echo "No .pcx files found in $io_dir"
  fi
  #exit 1
done

echo "All .pcx files have been processed."

