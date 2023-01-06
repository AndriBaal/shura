#! /bin/bash
# Script for setting up the android environment on arch linux

cargo install cargo-apk
rustup target add aarch64-linux-android

sudo pacman -S android-sdk-build-tools
sudo pacman -S android-sdk-cmdline-tools-latest
sudo pacman -S android-ndk

sudo /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager "platform-tools" "platforms;android-30"

export ANDROID_SDK_ROOT=/opt/android-sdk && export ANDROID_NDK_ROOT=/opt/android-ndk

echo "When using zsh maybe use: setopt +o nomatch"
echo "See Log: adb logcat RustStdoutStderr:D *:S"
