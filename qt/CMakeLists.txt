cmake_minimum_required(VERSION 3.24)

project(Medict VERSION 4.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Widgets Network WebEngineWidgets)
qt_standard_project_setup()

set(MEDICT_VERSION "${PROJECT_VERSION}" CACHE STRING "Medict display version")
set(MEDICT_BACKEND_BINARY "" CACHE FILEPATH "Path to the medictd backend binary")

set(MEDICT_SOURCES
    src/main.cpp
    src/api_client.cpp
    src/api_client.h
    src/definition_page.cpp
    src/definition_page.h
    src/main_window.cpp
    src/main_window.h
    src/bookmarks_dialog.cpp
    src/bookmarks_dialog.h
    src/settings_dialog.cpp
    src/settings_dialog.h
    src/css_editor_dialog.cpp
    src/css_editor_dialog.h
    resources/resources.qrc
)

if(WIN32)
    list(APPEND MEDICT_SOURCES resources/medict.rc)
endif()

qt_add_executable(medict WIN32 ${MEDICT_SOURCES})

if(WIN32)
    set_target_properties(medict PROPERTIES OUTPUT_NAME "Medict")
endif()

target_link_libraries(medict PRIVATE
    Qt6::Widgets
    Qt6::Network
    Qt6::WebEngineWidgets
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
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/linux/io.github.terasum.Medict.desktop"
        DESTINATION share/applications)
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/linux/io.github.terasum.Medict.png"
        DESTINATION share/icons/hicolor/512x512/apps
        RENAME io.github.terasum.Medict.png)
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/../packaging/linux/io.github.terasum.Medict.metainfo.xml"
        DESTINATION share/metainfo)
endif()

