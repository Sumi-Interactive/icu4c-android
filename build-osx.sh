#!/bin/bash

# Build OSX binaries:
OUTPUT_DIR="libs/osx"
rm -rf ${OUTPUT_DIR}
mkdir -p ${OUTPUT_DIR}

# Extract codes
#tar -xzvf download/icu4c-53_1-src.tgz
tar -xzvf download/icu4c-77_1-src.tgz
#cp ./download/icudt53l.dat ./icu/source/data/in/
cp ./download/icudt77l.dat ./icu/source/data/in/

cd ./icu/source

build_osx() {
    ARCH=$1
    echo "Building ICU for ${ARCH}"

    case $ARCH in
        arm64)
            export CFLAGS="-arch arm64"
            export CXXFLAGS="-arch arm64"
            export LDFLAGS="-arch arm64"
            ;;
        x86_64)
            export CFLAGS="-arch x86_64"
            export CXXFLAGS="-arch x86_64"
            export LDFLAGS="-arch x86_64"
            ;;
        x86)
            # 32-bit i386 (deprecated on modern macOS, may fail on M1)
            export CFLAGS="-arch i386 -m32"
            export CXXFLAGS="-arch i386 -m32"
            export LDFLAGS="-arch i386 -m32"
            ;;
        *)
            echo "Unsupported arch: ${ARCH}"
            return 1
            ;;
    esac

    CPPFLAGS="-DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1" \
    ./runConfigureICU MacOSX --enable-static --disable-shared

    make clean
    make -j8

    mkdir -p ../../${OUTPUT_DIR}/${ARCH}/
    cp lib/libicuuc.a   ../../${OUTPUT_DIR}/${ARCH}/
    cp lib/libicui18n.a ../../${OUTPUT_DIR}/${ARCH}/
    cp lib/libicudata.a ../../${OUTPUT_DIR}/${ARCH}/
}

# Build all 3 (arm64, x86_64, x86/i386)
build_osx arm64
build_osx x86_64
#build_osx x86

cd ../..
