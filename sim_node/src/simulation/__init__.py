from io import BytesIO

import gymnasium as gym
import numpy
import zenoh
from numpy import ndarray

terminated = False
truncated = False


def main() -> None:
    env = gym.make("InvertedPendulum-v5", render_mode="human")
    observation, info = env.reset()
    assert type(observation) is ndarray

    total_reward = None
    reward = None

    def take_action(action: zenoh.Sample):
        npbytes = action.payload.to_bytes()
        loaded_action = numpy.load(BytesIO(npbytes), allow_pickle=True)
        observation, reward, terminated, truncated, info = env.step(loaded_action)

    with zenoh.open(zenoh.Config()) as session:
        pub = session.declare_publisher("simulation/observation_space")
        sub = session.declare_subscriber("simulation/action_space", take_action)

        episode_over = False

        while not episode_over:
            # action = env.action_space.sample()
            # observation, reward, terminated, truncated, info = env.step(action)

            # Publish each observation if you want
            npbytes = BytesIO()
            numpy.save(npbytes, observation, allow_pickle=True)
            pub.put(npbytes.getvalue())
            # print(f"obs = {observation}, in bytes: {observation.tobytes()}")
            if reward is not None:  # essentially, if we have not yet taken an action
                total_reward += reward
                reward = None
            episode_over = terminated or truncated

        print(f"Episode finished! Total reward: {total_reward}")

    env.close()
