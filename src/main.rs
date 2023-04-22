use ncurses::*;

struct NcursesWindow {
    height: i32,
    width: i32,
    window: *mut i8,
    cursor_y: i32,
    cursor_x: i32,
}

impl NcursesWindow {
    fn new(
        height: i32,
        width: i32,
        y: i32,
        x: i32,
        border_attr: attr_t
    ) -> Self {
        let window = newwin(height, width, y, x);
        wbkgd(window, border_attr);
        NcursesWindow { height, width, window, cursor_y: 1, cursor_x: 1 }
    }

    fn destroy(& self) {
        delwin(self.window);
    }

    fn draw_border(& self) {
        box_(self.window, 0, 0);
        wrefresh(self.window);
    }

    fn write_char(&mut self, c: u32) {
        wmove(self.window, self.cursor_y, self.cursor_x);
        self.cursor_x += 1;
        if self.cursor_x == self.width - 1 {
            self.cursor_x = 1;
            self.cursor_y += 1;
        }
        if self.cursor_y == self.height - 1 {
            self.cursor_x = 1;
            self.cursor_y = 1;
        }
        waddch(self.window, c);
        wrefresh(self.window);
    }
}

struct NcursesWindows {
    win_left_top: NcursesWindow,
    win_left_bottom: NcursesWindow,
    win_middle: NcursesWindow,
    win_right_top: NcursesWindow,
    win_right_bottom: NcursesWindow,
}

impl NcursesWindows {
    fn new() -> Self {
        // Get screen dimensions
        let mut max_y = 0;
        let mut max_x = 0;
        getmaxyx(stdscr(), &mut max_y, &mut max_x);

        // Calculate window dimensions
        let half_height = max_y / 2;
        let side_width = (max_x) / 3;
        let top_height = half_height;
        let bottom_height = max_y - half_height;

        // Create windows
        let win_left_top = NcursesWindow::new(
            top_height, 
            side_width, 
            0, 
            0,
            COLOR_PAIR(1),
        );
        let win_left_bottom = NcursesWindow::new(
            bottom_height, 
            side_width, 
            top_height, 
            0,
            COLOR_PAIR(1),
        );
        let win_middle = NcursesWindow::new(
            max_y, 
            side_width, 
            0, 
            side_width,
            COLOR_PAIR(1),
        );
        let win_right_top = NcursesWindow::new(
            top_height, 
            side_width, 
            0, 
            side_width*2,
            COLOR_PAIR(1),
        );
        let win_right_bottom = NcursesWindow::new(
            bottom_height, 
            side_width, 
            top_height, 
            side_width*2,
            COLOR_PAIR(1),
        );

        NcursesWindows {
            win_left_top,
            win_left_bottom,
            win_middle,
            win_right_top,
            win_right_bottom,
        }
    }

    fn teardown(& self) {
        // Clean up and close ncurses
        self.win_left_top.destroy();
        self.win_left_bottom.destroy();
        self.win_middle.destroy();
        self.win_right_top.destroy();
        self.win_right_bottom.destroy();
    }

    fn screen_refresh(& self) {
        // Draw borders
        self.win_left_top.draw_border();
        self.win_left_bottom.draw_border();
        self.win_middle.draw_border();
        self.win_right_top.draw_border();
        self.win_right_bottom.draw_border();
    }

    fn write_char_to_middle(&mut self, c: u32) {
        // Refresh windows
        self.win_middle.write_char(c);
    }
}

fn main() {
    // Initialize ncurses
    initscr();
    start_color();
    raw();
    keypad(stdscr(), true);
    noecho();

    // Define color pairs
    init_pair(1, COLOR_WHITE, COLOR_BLACK);

    let mut windows = NcursesWindows::new();
    refresh();
    windows.screen_refresh();
    // Wait for user input
    let mut ch = getch();
    while ch != 0x0a {
        ch = getch();
        windows.write_char_to_middle(0x61);
    }
    windows.teardown();
    endwin();
}

