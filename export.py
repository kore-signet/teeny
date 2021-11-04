import json
import lmdb
import base64

env = lmdb.Environment("/db")
vals = {}
with env.begin() as txn:
    cursor = txn.cursor()
    for k,v in cursor.iternext():
        vals[base64.b64encode(k)] = base64.b64encode(v)

with open("out.json","w") as f:
    f.write(json.dumps(vals))