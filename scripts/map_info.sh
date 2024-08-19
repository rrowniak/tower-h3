#!/bin/bash -e 

RES_PROG=../src/target/debug/h3map
if [ ! -f $RES_PROG ]; then
    echo "Program $RES_PROG not found. Try building the workspace"
    exit 1
fi

input_dir="/home/rr/Games/H3/Maps"

for map_file in "$input_dir"/*.h3m; do
  if [ -f "$map_file" ]; then
    echo "Inspecting $map_file..."
    $RES_PROG show "$map_file"
  else
    echo "No .h3m files found in $input_dir"
  fi
done

echo "All .h3m files have been processed."

