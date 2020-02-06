import pickle
import json

table = pickle.load(open('hand_table.pkl', 'rb'))
json_table = {}
for tuple_key in table:
    string_key = ''.join(tuple_key)
    json_table[string_key] = table[tuple_key]

breakpoint()

json.dump(json_table, open('hand_table.json', 'w'))
