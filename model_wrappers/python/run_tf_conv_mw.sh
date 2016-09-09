#!/usr/bin/env bash

CLIPPER_MODEL_PATH=/crankshaw-local/tf_models/mnist_from_guilio/mnist_convnet/tf_checkpoint/convnet/convnet.ckpt \
  TF_CONV_BATCH_SIZE=256 \
  python tf_convnet_mnist_modelwrapper.py
