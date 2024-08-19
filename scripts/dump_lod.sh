#!/bin/bash -e 

RES_PROG=../src/target/debug/main
if [ ! -f $RES_PROG ]; then
    echo "Program $RES_PROG not found. Try building the workspace"
    exit 1
fi

input_dir="/home/rr/Games/H3/Data"
output_dir="/home/rr/Games/H3Dump/Data"

mkdir -p "$output_dir"

for lod_file in "$input_dir"/*.lod; do
  if [ -f "$lod_file" ]; then
    echo "Extracting $lod_file..."
    # Use the resources tool to extract the files into the output directory
    $RES_PROG dump "$lod_file" "$output_dir"
    echo "$lod_file extracted successfully!"
  else
    echo "No .lod files found in $input_dir"
  fi
done

echo "All .lod files have been processed."

