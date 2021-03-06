from __future__ import print_function
import sys
import os
sys.path.append(os.path.abspath("../pyclipper/src"))
import json
# import requests
import random
from datetime import datetime

import pandas as pd
import numpy as np
import sklearn.linear_model

def load_digits(digits_location, digits_filename = "train-mnist-dense-with-labels.data"):
    digits_path = digits_location + "/" + digits_filename
    print("Source file:", digits_path)
    df = pd.read_csv(digits_path, sep=",", header=None)
    data = df.values
    print("Number of image files:", len(data))
    y = data[:,0]
    X = data[:,1:]
    return (X, y)

def normalize_digits(X):
    mu = np.mean(X,0)
    sigma = np.var(X,0)
    Z = (X - mu) / np.array([np.sqrt(z) if z > 0 else 1. for z in sigma])
    return Z 


def mnist_prediction(x, uid):
    url = "http://127.0.0.1:1337/predict?uid=%(uid)s" % {'uid': uid}
    headers = {}
    # x_str = ", ".join(["%.3f" % a for a in x])
    x_str = ", ".join(["%d" % a for a in x])
    # print(x_str)
    start = datetime.now()
    r = requests.post(url, data=x_str, headers=headers)
    end = datetime.now()
    latency = (end - start).total_seconds() * 1000.0
    print("'%s', %f ms" % (r.text, latency))

if __name__=='__main__':
    args = sys.argv
    x, y = load_digits(os.path.expanduser("~/model-serving/data/mnist_data"), digits_filename = "test.data")
    # z = normalize_digits(x)
    mnist_prediction(x[int(args[1])],int(args[2]))
