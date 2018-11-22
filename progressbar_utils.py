import progressbar

def init_progress_bar(iterations):
    widgets = [' ', progressbar.Percentage(),
               ' ', progressbar.Bar(),
               ' ', progressbar.ETA()]
    return progressbar.ProgressBar(widgets=widgets, max_value=iterations)
