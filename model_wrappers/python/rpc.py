from __future__ import print_function
import array
import struct
import SocketServer
import numpy as np
import time
import datetime
from datetime import datetime
import sys
import os

SHUTDOWN_CODE = 0
FIXEDINT_CODE = 1
FIXEDFLOAT_CODE = 2
FIXEDBYTE_CODE = 3
VARINT_CODE = 4
VARFLOAT_CODE = 5
VARBYTE_CODE = 6
STRING_CODE = 7

TIMING_ON = True

RECV_SIZE = 4096

# class NoopModelWrapper(ModelWrapperBase):
#
#
#     def __init__(self):
#         print("NoopModelWrapper init code running")
#
#
#     def predict_ints(self, inputs):
#         print("predicting %d integer inputs" % len(inputs))
#         return np.arange(1,len(inputs) + 1)
#
#     def predict_floats(self, inputs):
#         print("predicting %d float inputs" % len(inputs))
#         return np.arange(1,len(inputs) + 1)

class ModelWrapperBase(object):
    def predict_ints(self, inputs):
        pass

    def predict_floats(self, inputs):
        pass

    def predict_bytes(self, inputs):
        pass

    def predict_strings(self, inputs):
        pass

def is_fixed_format(fmt):
    return fmt == FIXEDINT_CODE or fmt == FIXEDFLOAT_CODE or fmt == FIXEDBYTE_CODE

def is_var_format(fmt):
    return fmt == VARINT_CODE or fmt == VARFLOAT_CODE or fmt == VARBYTE_CODE

class ClipperRpc(SocketServer.BaseRequestHandler):

    allow_reuse_address = True

    # def __init__(self):
        # self.allow_reuse_address = True



    def handle(self):
        print("HANDLING NEW CONNECTION kjsdhfjkdhs", file=sys.stderr)
        recv_time_buf = []
        while True:
            if len(recv_time_buf) >= 1000:
                print("Mean recv time: %f microseconds" % np.mean(np.array(recv_time_buf)), file=sys.stderr)
                recv_time_buf = []
            header_bytes = 5
            data = ""
            # self.request.settimeout(0.5)
            # self.request.setblocking(1)
            # wait for header
            while len(data) < header_bytes:
                data += self.request.recv(4096)

            t1 = datetime.now()
            header, data = (data[:header_bytes], data[header_bytes:])
            input_type, num_inputs = struct.unpack("<BI", header)
            if input_type == SHUTDOWN_CODE:
                print("Shutting down connection", file=sys.stderr)
                self.request.sendall(np.array([1234]).astype('uint32').tobytes())
                return
            if is_fixed_format(input_type):
                additional_header_bytes = 4
                while len(data) < additional_header_bytes:
                    data += self.request.recv(RECV_SIZE)
                input_len = struct.unpack("<I", data[:additional_header_bytes])[0]
                data = data[additional_header_bytes:]
                inputs = []
                if input_type == FIXEDBYTE_CODE:
                    total_bytes_expected = input_len*num_inputs
                    while len(data) < total_bytes_expected:
                        data += self.request.recv(RECV_SIZE)
                    input_bytes = np.array(array.array('B', bytes(data)))
                    inputs = np.split(input_bytes, num_inputs)
                    for i in inputs:
                        assert len(i) == input_len
                elif input_type == FIXEDFLOAT_CODE:
                    total_bytes_expected = 8*input_len*num_inputs
                    while len(data) < total_bytes_expected:
                        data += self.request.recv(RECV_SIZE)
                        # print("Received %d bytes of %d expected" % (len(data), total_bytes_expected))
                    # input_doubles = np.array(array.array('d', bytes(data[:total_bytes_expected])))
                    input_doubles = np.array(array.array('d', bytes(data)))
                    inputs = np.split(input_doubles, num_inputs)
                    for i in inputs:
                        assert len(i) == input_len
                else:
                    assert input_type == FIXEDINT_CODE
                    total_bytes_expected = 4*input_len*num_inputs
                    while len(data) < total_bytes_expected:
                        data += self.request.recv(RECV_SIZE)
                    input_ints = np.array(array.array('i', bytes(data)))
                    inputs = np.split(input_ints, num_inputs)
                    for i in inputs:
                        assert len(i) == input_len

            elif is_var_format(input_type):
                raise NotImplementedError
            elif input_type == STRING_CODE:
                raise NotImplementedError
            else:
                raise RuntimeError("Invalid input type: %d" % input_type)

            t2 = datetime.now()
            recv_time_buf.append((t2 - t1).microseconds)

            if input_type == FIXEDINT_CODE or input_type == VARINT_CODE:
                predictions = self.server.model.predict_ints(inputs)
            elif input_type == FIXEDFLOAT_CODE or input_type == VARFLOAT_CODE:
                predictions = self.server.model.predict_floats(inputs)
            elif input_type == FIXEDBYTE_CODE or input_type == VARBYTE_CODE:
                predictions = self.server.model.predict_bytes(inputs)
            else:
                predictions = self.server.model.predict_strings(inputs)
            assert len(predictions) == num_inputs
            assert predictions.dtype == np.dtype('float64')
            self.request.sendall(predictions.tobytes())




def start(model_wrapper, port):
    ip = "0.0.0.0"
    # ip = "localhost"
    server = SocketServer.TCPServer((ip, port), ClipperRpc)
    server.model = model_wrapper
    # server.handle_request()
    print("Starting to serve", file=sys.stderr)
    server.serve_forever()



