# cwver
## Abstract
`cwver` is a command line tool to work with __calender week version__ strings in the following form

    <yy>w<ww>.<d>

For example `21w01.1` corresponds to monday in the 1st week of 2021. The corresponding ISO date is 2021-01-04.

Valid values:
- `<yy>` represents the year in two digits
- `<ww>` represents the ISO calendar week in two digits and can have a range from 01 - 53.
- `<d>` can have a range from 1 (=monday) till 7 (=sunday).

More about ISO week date: https://en.wikipedia.org/wiki/ISO_week_date

## Usage

`cwver` supports 3 major subcommands:

    cwver 0.1.0
    Florian Bramer <elektronenhirn@gmail.com>
    Command line tool to work with calendar week version strings (e.g. 21w45.7).

    USAGE:
        cwver [SUBCOMMAND]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    SUBCOMMANDS:
        bisect     Calculates the workday(s) in the middle of two given cw versions     spanning a regression range.
                   Saturdays and sundays are ignored. Use --workdays to override.
        convert    Convert cw version string (e.g. 21w45.7) into ISO date.
        help       Prints this message or the help of the given subcommand(s)
        today      Display today's date as cw version string.

### convert

Converts a calendar week version string into an ISO date. E.g.:

    ✗ cwver convert 21w05.6
    21w05.6 = 2021-02-06

Or the other way around:

    ✗ cwver convert 2021-02-06
    2021-02-06 = 21w05.6

### today

Prints today's date in the calender week format. E.g.

    ✗ cwver today
    Today = 21w11.1

### bisect

Large software projects often provide one nightly build per day. You might receive bug reports similar to:

> I observed a regression in our software: The nightly build 21w04.3 is affected. It was still working fine back in 21w03.1.

You might want to know now: which was the first nightly build that introduced the regression?

The information given above spans your regression range of

`21w03.1 ➔ 21w04.3`

The range contains 7 workdays (we want to ignore saturday and sunday). `cwver bisect` can help you to find the middle in the range to start your binary search journey:


    ✗ cwver bisect 21w03.1 21w04.3
    Regression Range:
     2021-01-18  ➔  2021-01-27 (7 workday(s))

    Two equivaletent bisect starting points:
     • 21w03.4 = 2021-01-21, or
     • 21w03.5 = 2021-01-22

In this example there is not even single nightly build in the middle. The middle of the range is two days wide. So it is up to you to pick one.

#### Workdays
`cwver` ignores saturday and sundays per default. You can override this default behaviour with the `--workdays` option.

