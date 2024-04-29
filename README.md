# pingora_ingress

Use the pingora framework as the gateway to k8s. It monitors changes to ingress and updates automatically.

## Quick start
1. Create ServiceAccount:`ring-sa`,cmd:`kubectl apply -f ./deploy/role.yaml -n qa`.
2. Create Deployment,docker:`wdshihaoren/pingora-ingress:v0.1.0`
3. Create Ingress
4. Test

[Reference blog]()

## Plan

This is only an early version, and it will be improved in the future

## License

This project is licensed under the Apache 2.0 general use license. You're free to integrate, fork, and play with this code as you feel fit without consulting the author, as long as you provide proper credit to the author in your works.

