# Test code for blotto

from blotto import *
import numpy as np

trainer = Trainer()
trainer.train(50000)
assert(utility(ACTIONS[0], ACTIONS[1]) == 0)
assert(utility(ACTIONS[1], ACTIONS[0]) == 0)
assert(utility(ACTIONS[3], ACTIONS[0]) == 1)
assert(utility(ACTIONS[0], ACTIONS[3]) == -1)
print(np.sum(trainer.get_average_strategy()))
strat = trainer.get_average_strategy()
for i, a in enumerate(ACTIONS):
    print('%s: %f' % (a, strat[i]))