import json
import lmdb
import base64
import unpaddedbase64

env = lmdb.Environment("/db")
vals = {"hashes": {}, "urls": {}}
with env.begin() as txn:
    cursor = txn.cursor()
    for k,v in cursor.iternext():
        if len(k) == 32:
            vals["hashes"][base64.b64encode(k).decode()] = unpaddedbase64.encode_base64(v,True)
        else:
            vals["urls"][base64.b64encode(k).decode()] = base64.b64encode(v).decode()
#            print("hash")

#            print(v)
#        vals[base64.b64encode(k).decode()] = base64.b64encode(v).decode()

with open("out.json","w") as f:
    f.write(json.dumps(vals))