import os
import itertools
import pickle

TABLE_NAME = 'hand_table.pkl'

class HandTable:

    def __init__(self):
        if os.path.isfile(TABLE_NAME):
            self.table = pickle.load(open(TABLE_NAME, 'rb'))
        else:
            self.table = self.make_table()
            pickle.dump(self.table, open(TABLE_NAME, 'wb'))

    def __getitem__(self, cards):
        raise NotImplementedError

