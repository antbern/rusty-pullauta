# Karttapullautin
### ***Map Generator***

Karttapullautin is an application that is designed to generate highly accurate maps out of LiDAR data input files. Built using the Rust programming language, Karttapullautin takes advantage of Rust's performance to deliver fast results on Linux, Mac and Windows.

Karttapullautin supports a wide range of LiDAR data input file formats, namely LAS, LAZ, and XYZ files. The software includes advanced algorithms for filtering, classification, and feature extraction, ensuring that users can generate highly accurate maps with ease.

Due to its performance and accuracy, with its powerful features and fast results, Karttapullautin is a must-have tool for anyone willing to generate orienteering maps from LiDAR data.

## Usage

Download the latest binary for your platform from https://github.com/rphlo/karttapullautin/releases/latest and extract the files where you want to use them.

### Compiling code from source

If your platform is not listed you must compile the binary.

You'll need to install the rust toolchain.

See https://rustup.rs  

Then download the latest code at https://github.com/rphlo/karttapullautin/releases/latest and compile it.

    cargo build --release

The `pullauta` binary will be accessible in the `target/release/` directory. You can proceed and copy it to your desired directory.

### Converting a LiDAR file

Karttapullautin accepts .LAS, .LAZ or .XYZ file with classification (xyzc).

You can run the `pullauta` executable with the path to your file as argument:  
    
    ./pullauta L3323H3.laz

> Note: By defaut messages with the log level _info_ will be printed to the console. To show more information (eg. timings of each operation),
> set the `RUST_LOG` environment variable to `debug` or specify it on the command line like so:
> ```bash
> RUST_LOG=debug ./pullauta [..]
> ```

As output Karttapullautin writes two 600 dpi png map images. One without depressions and one with purple depressions. It also writes contours and cliffs as dxf files to temp folder to be post processed, for example using Open Orienteering Mapper or OCAD.

You can re-render png map files (like with changed north line settings) by running the binary without arguments.  
    
    ./pullauta

Karttapullautin can also render zip files containing shape files downloaded from differents sources. After normal process just run the binary with the zip(s) as arguments. You must define your configuration file describing the shape file content, in the ini file, parameter `vectorconf` (see osm.txt and fastighetskartan.txt).

    ./pullauta yourzipfile1.zip yourzipfile2.zip yourzipfile3.zip yourzipfile4.zip

For Finns: Karttapullautin render Maastotietokanta zip files (shape files) downloaded from the download site of Maanmittauslaitos without setting a configuration file. Just leave `vectorconf` parameter empty.

To print a map at right scale, you download for example IrfanView http://www.irfanview.com/ open png map, Image -> Information, set resolution 600 x 600 DPI and push "change" button and save.  Then crop map if needed (Select area with mouse and Edit -> crop selection). Print using "Print size: Original Size srom DPI". Like this your map should end up 1:10000 scale on paper.

### Fine tuning the output

`pullauta` creates a `pullauta.ini` file if it doesn't already exists. Your settings are there. For the second run you can change settings as you wish. Experiment with small file to find best settings for your taste/terrain/lidar data.

For Ini file configuration explanation, see ini file comments.

### Re-processing steps again

When the process is done and you find there is too much green or too small cliffs, you can make parts of the process again with different parameters without having to do it all again. To re-generate only vegetation type from command line:

    ./pullauta makevege
    ./pullauta 

To make cliffs again:

    ./pullauta makecliffs xyztemp.xyz 1.0 1.15
    ./pullauta

### Vectors

In additon to the png raster map imges, Karttapullautin makes also vector contours and cliffs and also some raster vector files one might find intresting for mapping use. After the process you can find them in temp folder.

- `out2.dxf`: final contours with 2.5 m interval
- `dotknolls.dxf`: dot knolls and small U -depressions. Some are not rendered to png files for legibility reasons.
- `c1g.dxf`: small cliffs
- `c2g.dxf`: big cliffs
- `vegetation.png + vegetation.pgw`: generalized green/yellow as raster, same as at the background of final map png files.

For importing Maastotietokanta, try reading shape filed directly to your mapping app..

### Batch processing

Karttapulautin can also batch process all las/las files + Maastotietokanta zips in a directory. To do it, turn batch processing on in ini file. configure your input file directory and output directory for map tiles. Copy your input files to input directory and run `./pullauta`. It starts processing las/laz files one by one until everything is done. If you have several cores 
in your CPU, you can make use of all of them to process multiple file at once. you can configure it with `processes` parameter in ini file. Note, processes parameter effects only batch mode, in normal mode it uses just one worker process. You will also need lots of RAM to process simultaneously several large laser files. To re-process tiles in bach mode you need to remove previous png files from output folder.

You can merge png files in output folder with Karttapullautin.

Without the depressions

    ./pullauta pngmerge 1

and depression versions

    ./pullauta pngmergedepr 1

vegetation backround images (if saved, there is parameter for saving there)

    ./pullauta pngmergevege


The last paramameter (number) is scale factor. 2 reduces size to 50%, 4 to 25%, 20 to 5% and so on. Command writes out jpg and png versions. 
Note, you easily run out of memory if you try merging together too large area with too high resolution.

You can also merge dxf files (if saved, there is parameter for saving there)

    ./pullauta dxfmerge

### Note:

Some commands from the original perl karttapullatin that are either obsolete or not necessary for the map generation are not supported by this new rust version:  

They are:
  - `cliffgeneralize`
  - `ground`
  - `ground2`
  - `groundfix`
  - `makecliffsold`
  - `makeheight`
  - `vege`
  - `profile`
  - `xyzfixer`

If you need to run one of those, you must use the original perl script https://www.routegadget.net/karttapullautin/ or https://github.com/linville/kartta-pack for mac and linux

## Development

Make your changes, then youd run:

    cargo build --release

The new binary will be accessible in the `target/release/` directory
