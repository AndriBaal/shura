#! /bin/bash
# Script for setting up the android environment on arch linux

cargo install cargo-apk
rustup target add aarch64-linux-android

sudo apt install -y android-sdk-build-tools android-sdk-platform-tools-common adb
sudo sdkmanager "platform-tools" "platforms;android-30" "build-tools;30.0.3" "ndk;23.1.7779620"
sudo usermod -aG plugdev $LOGNAME

export ANDROID_HOME=/opt/android-sdk && export ANDROID_SDK_ROOT=/opt/android-sdk && export ANDROID_NDK_ROOT=/opt/android-sdk/ndk/23.1.7779620

echo "You might need to log out for the changes to take effect"
echo "When using zsh maybe use: setopt +o nomatch"
echo "See Log with: adb logcat RustStdoutStderr:D *:S"
