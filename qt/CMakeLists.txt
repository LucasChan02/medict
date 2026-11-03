cmake_minimum_required(VERSION 3.30)

file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/../VERSION" MEDICT_FILE_VERSION LIMIT_COUNT 1)
string(STRIP "${MEDICT_FILE_VERSION}" MEDICT_FILE_VERSION)
if(NOT MEDICT_FILE_VERSION MATCHES "^[0-9]+\\.[0-9]+\\.[0-9]+$")
    message(FATAL_ERROR "VERSION must contain a three-segment version, got '${MEDICT_FILE_VERSION}'")
endif()

project(Medict VERSION ${MEDICT_FILE_VERSION} LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 6.10 REQUIRED COMPONENTS
    Core
    Gui
    Network
    Qml
    Quick
    QuickControls2
    Svg
    WebEngineQuick
)
find_package(KF6Kirigami 6.0 REQUIRED)

qt_standard_project_setup(REQUIRES 6.10)

set(MEDICT_VERSION "${PROJECT_VERSION}" CACHE STRING "Medict display version")
set(MEDICT_BACKEND_BINARY "" CACHE FILEPATH "Path to the medictd backend binary")

set(MEDICT_SOURCES
    src/main.cpp
    src/api_client.cpp
    src/api_client.h
    src/app_controller.cpp
    src/app_controller.h
    resources/resources.qrc
)

if(WIN32)
    list(APPEND MEDICT_SOURCES resources/medict.rc)
endif()

qt_add_executable(medict WIN32 ${MEDICT_SOURCES})

qt_add_qml_module(medict
    URI Medict
    VERSION 1.0
    QML_FILES
        qml/Main.qml
        qml/LookupPage.qml
        qml/NotebooksPage.qml
        qml/SettingsPage.qml
        qml/CssEditorPage.qml
        qml/AboutPage.qml
)

if(WIN32)
    set_target_properties(medict PROPERTIES OUTPUT_NAME "Medict")
endif()

target_link_libraries(medict PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Network
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Svg
    Qt6::WebEngineQuick
    KF6::Kirigami
)

target_compile_definitions(medict PRIVATE MEDICT_VERSION="${MEDICT_VERSION}")

if(MSVC)
    target_compile_options(medict PRIVATE /W4 /permissive-)
else()
    target_compile_options(medict PRIVATE -Wall -Wextra -Wpedantic)
endif()

if(WIN32)
    install(TARGETS medict RUNTIME DESTINATION .)
    if(MEDICT_BACKEND_BINARY)
        install(PROGRAMS "${MEDICT_BACKEND_BINARY}" DESTINATION . RENAME medictd.exe)
    endif()
else()
    install(TARGETS medict RUNTIME DESTINATION bin)
    if(MEDICT_BACKEND_BINARY)
        install(PROGRAMS "${MEDICT_BACKEND_BINARY}" DESTINATION bin RENAME medictd)
    endif()
    install(FILES
        "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/linux/io.github.terasum.Medict.desktop"
        DESTINATION share/applications)
    install(FILES
        "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/linux/io.github.terasum.Medict.png"
        DESTINATION share/icons/hicolor/512x512/apps
        RENAME io.github.terasum.Medict.png)
    install(FILES
        "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/linux/io.github.terasum.Medict.metainfo.xml"
        DESTINATION share/metainfo)
endif()

install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/../LICENSE"
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/icons/LICENSE-LGPL-2.1.txt"
    DESTINATION share/licenses/medict)
