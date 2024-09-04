import requests

# Fonction ranksw pour récupérer les données de l'API swrt
def infoRankSW():
    url = "https://m.swranking.com/api/player/nowline"
    response = requests.get(url)

    if response.status_code == 200:
        data = response.json()

        scores = {
            'c1': data['data']['c1']['score'],
            'c2': data['data']['c2']['score'],
            'c3': data['data']['c3']['score'],
            'p1': data['data']['s1']['score'],
            'p2': data['data']['s2']['score'],
            'p3': data['data']['s3']['score'],
            'g1': data['data']['g1']['score'],
            'g2': data['data']['g2']['score'],
            'g3': data['data']['g3']['score']
        }

        return scores
    else:
        return None

# Fonction permettant de récupérer les infos via l'api swarena
def infoPlayerSwarena(id: int):
    url = f"https://api.swarena.gg/player/{id}/seasons"
    response = requests.get(url)
    data = response.json()

    if "data" in data and data["data"] is not None:
        available_seasons = data["data"]
        player_data = []

        for season in available_seasons:
            url = f"https://api.swarena.gg/player/{id}/summary?season={season}"
            response = requests.get(url)
            data = response.json()

            if "error" not in data and data.get("data") is not None:
                wizard_data = data["data"]
                wizard_name = wizard_data.get("wizard_name", "N/A")
                wizard_country = wizard_data.get("wizard_country", "N/A")
                wizard_picture = wizard_data.get("wizard_picture", "N/A")

                player_data.append({
                    "season": season,
                    "name": wizard_name,
                    "country": wizard_country,
                    "picture": wizard_picture
                })

        return player_data
    else:
        return None