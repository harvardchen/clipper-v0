#include<vector>
#include <memory>
#include "ClipperRPC.h"
#include "Model.h"

class NoopModelWrapper : public Model {
    public:
        vector<double> predictions;

        vector<double>& predict_bytes(vector< vector<char> >& input);

        vector<double>& predict_floats(vector< vector<double> >& input);

        vector<double>& predict_ints(vector< vector<uint32_t> >& input);

        vector<double>& predict_strings(vector< vector<string> >& input);
};

vector<double>& NoopModelWrapper::predict_bytes(
        vector<vector<char> >& input) {
    int i, j;
    double total;
    predictions.resize(input.size());
    for (i = 0; i < input.size(); i++) {
        total = 0;
        for (j = 0; j < input[i].size(); j++) {
            total += input[i][j];
        }
        predictions[i] = total;
    }
    return predictions;
}

vector<double>& NoopModelWrapper::predict_floats(
        vector<vector<double> >& input) {
    int i, j;
    double total;
    predictions.resize(input.size());
    for (i = 0; i < input.size(); i++) {
        total = 0;
        for (j = 0; j < input[i].size(); j++) {
            total += input[i][j];
        }
        predictions[i] = total;
    }
    return predictions;
}

vector<double>& NoopModelWrapper::predict_ints(
        vector<vector<uint32_t> >& input) {
    int i, j;
    double total;
    predictions.resize(input.size());
    for (i = 0; i < input.size(); i++) {
        total = 0;
        for (j = 0; j < input[i].size(); j++) {
            total += input[i][j];
        }
        predictions[i] = total;
    }
    return predictions;
}

vector<double>& NoopModelWrapper::predict_strings(
        vector<vector<std::string> >& input) {
    int i;
    predictions.resize(input.size());
    for (i = 0; i < input.size(); i++) {
        predictions[i] = input[i].size();
    }
    return predictions;
}

int main() {
    std::unique_ptr<Model> model(new NoopModelWrapper());
    ClipperRPC *clipper_rpc_server =
        new ClipperRPC(model, (char *) "0.0.0.0", 6001);
    clipper_rpc_server->serve_forever();
}
