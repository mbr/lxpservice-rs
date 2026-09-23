lxpservice-rs - a command line tool to operate LetterXpress web service.
=

Letterxpress (https://www.letterxpress.de/) provides a service using a web API to easily use printing services. PDF documents can be transferred to be printed and sent by Letterxpress. Not only is this convenient, but it is also offered at an amazingly low price.

The command line tool lxp makes it possible to use this web service with a command line tool. This tool is written in rust and therefore platform neutral, if it has only been tested under Linux so far.

## Development

The development environment uses the `github:mbr/flakes#rust` template.
Run `direnv allow` to load the pinned Rust toolchain and native dependencies.
Use `./check.sh` for checks and tests, `./format.sh` for formatting, and
`nix build` to build the CLI at `result/bin/lxp`.

Copy `.env.example` to `.env` and fill in your credentials locally. Direnv
loads `.env` into the shell; `.env` and its variants are ignored by Git and
must never be committed. The existing CLI still uses its legacy profile
configuration: the `LXP_*` variables are reserved for the API v3 migration
and are not yet consumed by the application.

The possibilities of the tool are presented below.

Getting help
````
$ lxp --help
lxp 0.1
Winfried Simon <winfried.simon@gmail.com>
Command line tool to manage LetterXpress print jobs

USAGE:
    lxp [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Be communicative

SUBCOMMANDS:
    help       Prints this message or the help of the given subcommand(s)
    invoice    Handle invoices
    job        Print job handling
    profile    Create and maintain profiles
    set        Set print job(s) on server

````
Getting help to subcommand
-
````
$ lxp profile --help
lxp-profile 
Create and maintain profiles

USAGE:
    lxp profile [FLAGS] [ARGS]

FLAGS:
    -d, --delete        Delete a single profile
    -a, --delete_all    Delete all profiles
    -h, --help          Prints help information
    -n, --new           Create and select a new profile
    -o, --overview      Show all profiles
    -s, --switch        Switch to profile
    -V, --version       Prints version information

ARGS:
    <profile>    Name of user profile
    <user>       User name of print service
    <url>        Url to print service
    <api_key>    Api key of print service

A profile has a name and contains all information for accessing the web
service. With the subcommand profile they can be displayed, created and
deleted. You can also switch between them. 
````
This example shows the help text for the sub command profile. There are also help screens for the sub commands invoice, job and set available.

User profile handling
- 

Adde a new profile to the profile registry.
````
$ lxp profile -n <profile_name> <user_name> <url api_key>
````

Delete a user profile
````
$ lxp profile -d <profile_name>
````

Delete all user profiles
````
$ lxp profile -a
````

Switch to a given profile
````
$ lxp profile -s <profile_name>
````

Show all profiles
````
$ lxp profile -o
Active profile 'profile1'

<profile>       <user>              <url>
profile1        user1               url1
profile2        user2               url2
profile3        user3               url3
````

Show and download invoices
-
Download current invoice
````
$ lxp invoice -c
Writing file '2020-10-31_profile-invoice.pdf'
````

Show list of invoices
````
$ lxp invoice -l

Date           Id     Cost
2020-10-31  30711 149.98 €
2019-01-31  11328  27.12 €
2019-09-30  16844 107.27 €
````

Show and delete print jobs
-
List all print jobs on server
````
$ lxp job -o
ctive profile 'profile1'
Credit balance 98.14 €

These letters will be sent soon:

Date           Id Pgs Col Dpx Shp Cost Filename                           
2020-12-10  57451   1   4 sim nat 0.93 letter1.pdf                        
2020-12-10  57452   1   4 sim nat 0.93 letter2.pdf                        
The sum of the costs is 1.86 €

These letters are in the queue (credit exhausted):
<No data>

These letters are sent in the last 7 days:
<No data>
````
Delete print job by id
````
$ lxp job -d -i 57451
  Job id 57451  deleted
````

Delete all print jobs
````
$ lxp job -d -a
  Job id 57452 letter1.pdf deleted
  Job id 57454 letter2.pdf deleted
  Job id 57453 letter3.pdf deleted
3 job(s) deleted
````
Upload print jobs to the web service
-
Upload a single pdf file
````
$ lxp set letter1.pdf 
  Job letter1.pdf sent
````
Upload all pdf files of a directory
````
$ lxp set pdf_dir
  Job pdf_dir/letter3.pdf sent
  Job pdf_dir/letter5.pdf sent
  Job pdf_dir/letter4.pdf sent
  Job pdf_dir/letter2.pdf sent
  Job pdf_dir/letter1.pdf sent
````
