from __future__ import print_function
import sys
import yaml
import toml


cur_model_core_num = 0
MAX_CORES = 47
isolated_cores = True

def reserve_cores(num_cores):
    global cur_model_core_num
    s = "%d-%d" % (cur_model_core_num, cur_model_core_num + num_cores - 1)
    cur_model_core_num += num_cores
    if cur_model_core_num >= MAX_CORES:
        print("WARNING: Trying to reserve more than 48 cores: %d" % cur_model_core_num)
    return s

def overlap_reserve_cores():
    s = "%d-%d" % (cur_model_core_num, MAX_CORES)
    return s


clipper_conf_dict = {
        "name" : "clipper-test",
        "slo_micros" : 20000,
        "num_message_encodes" : 1,
        "correction_policy" : "logistic_regression",
        "use_lsh" : False,
        "input_type" : "float",
        "input_length" : 784,
        "window_size" : -1,
        "redis_ip" : "redis",
        "redis_port" : 6379,
        "results_path" : "/tmp/benchmarking_logs",
        "num_predict_workers" : 16,
        "num_update_workers" : 1,
        "cache_size" : 49999,
        "mnist_path" : "/mnist_data/test.data",
        "num_benchmark_requests" : 1000000,
        "target_qps" : 40000,
        "bench_batch_size" : 300,
        "salt_cache" : True,
        "batching": { "strategy": "learned", "sample_size": 1000},
        "models": []
        }



dc_dict = {
        "version": "2",
        "services": {
            "redis": {"image": "redis:alpine", "cpuset": reserve_cores(1)},
            "clipper": {"image": "cl-dev",
                "cpuset": reserve_cores(8),
                "depends_on": ["redis"],
                "volumes": [
                    "${MNIST_PATH}:/mnist_data:ro",
                    "${CLIPPER_ROOT}/digits_bench.toml:/tmp/digits_bench.toml:ro",
                    "${CLIPPER_ROOT}/benchmarking_logs/thruput_mw_debug:/tmp/benchmarking_logs"
                    ],
                }
            }
        }




def add_model(name_base, image, mp, container_mp, num_replicas=1):
    model_names = [name_base + "_r%d" % i for i in range(num_replicas)]
    model_addrs = ["%s:6001" % n for n in model_names]
    clipper_model_def = {
            "name": name_base,
            "addresses": model_addrs,
            "num_outputs": 1,
            "version": 1
    }
    dc_entries = {}
    for n in model_names:
        if isolated_cores:
            core_res = reserve_cores(1)
        else:
            core_res = overlap_reserve_cores()

        dc_entries[n] = {
                "image": image,
                "volumes": ["%s:/model:ro" % mp],
                "environment": ["CLIPPER_MODEL_PATH=%s" % container_mp],
                "cpuset": core_res,
                }

    # dc_entries should be added to the depends_on list

    global clipper_conf_dict
    global dc_dict
    clipper_conf_dict["models"].append(clipper_model_def)
    dc_dict["services"]["clipper"]["depends_on"].extend(model_names)
    dc_dict["services"].update(dc_entries)
    # return (clipper_model_def, model_names, dc_entries)

def add_sklearn_rf(depth, num_replicas=1):
    name_base = "rf_d%d" % depth
    image = "clipper/sklearn-mw"
    mp = "${CLIPPER_ROOT}/model_wrappers/python/sklearn_models/50rf_pred3_depth%d/" % depth
    container_mp =  "/model/50rf_pred3_depth%d.pkl" % depth
    add_model(name_base, image, mp, container_mp, num_replicas)

def add_sklearn_log_regression(num_replicas=1):
    name_base = "logistic_reg"
    image = "clipper/sklearn-mw"
    mp = "${CLIPPER_ROOT}/model_wrappers/python/sklearn_models/log_regression_pred3/",
    container_mp = "/model/log_regression_pred3.pkl"
    add_model(name_base, image, mp, container_mp, num_replicas)

def add_sklearn_linear_svm(num_replicas=1):
    name_base = "linear_svm"
    image = "clipper/sklearn-mw"
    mp = "${CLIPPER_ROOT}/model_wrappers/python/sklearn_models/linearsvm_pred3/",
    container_mp = "/model/linearsvm_pred3.pkl"
    add_model(name_base, image, mp, container_mp, num_replicas)

def add_sklearn_kernel_svm(num_replicas=1):
    name_base = "kernel_svm"
    image = "clipper/sklearn-mw"
    mp = "${CLIPPER_ROOT}/model_wrappers/python/sklearn_models/svm_pred3/",
    container_mp = "/model/svm_pred3.pkl"
    add_model(name_base, image, mp, container_mp, num_replicas)

def add_noop(num_replicas=1):
    name_base = "noop"
    image = "clipper/noop-mw"
    # these values don't matter
    model_path = "${CLIPPER_ROOT}/model_wrappers/python/sklearn_models/linearsvm_pred3/",
    container_mp = "/model/log_regression_pred3.pkl"


def add_spark_svm(num_replicas=1):
    name_base = "spark_svm"
    image = "clipper/spark-mw-dev"
    mp = "${CLIPPER_ROOT}/model_wrappers/python/spark_models/svm_predict_3",
    container_mp ="/model"
    add_model(name_base, image, mp, container_mp, num_replicas)

if __name__=='__main__':

    ## SKLEARN RF
    add_spark_svm(num_replicas=5)
    add_sklearn_linear_svm(num_replicas=5)
    add_sklearn_log_regression(num_replicas=5)
    add_sklearn_rf(depth=16, num_replicas=7)

    print("CORES USED: %d" % cur_model_core_num)

    # with open("../digits_bench_TEST.toml", 'w') as f:
    with open("../digits_bench.toml", 'w') as f:
        toml.dump(clipper_conf_dict, f)

    with open("docker-compose.yml", 'w') as f:
        yaml.dump(dc_dict, f)





# vim: tabstop=8 expandtab shiftwidth=4 softtabstop=4